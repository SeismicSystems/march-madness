//! Market-making edge computation against Kalshi orderbooks.
//!
//! Measures how much money our model would make trading against the orderbook.
//! An efficient (well-calibrated) model makes $0 — any residual edge means
//! the model disagrees with the market.

use crate::types::{Orderbook, TeamOrderbook};
use std::collections::HashMap;
use std::fmt;

/// Which side of the trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Side::Buy => write!(f, "BUY"),
            Side::Sell => write!(f, "SELL"),
        }
    }
}

/// Edge breakdown for one team/round market.
#[derive(Debug, Clone)]
pub struct MarketEdge {
    pub team: String,
    pub round: usize,
    pub ticker: String,
    pub model_prob: f64,
    /// Edge from buying YES (model thinks team is underpriced).
    pub buy_edge: f64,
    /// Edge from selling YES (model thinks team is overpriced).
    pub sell_edge: f64,
    /// Total edge = buy_edge + sell_edge.
    pub total_edge: f64,
    /// Signed edge for gradient: sell_edge - buy_edge.
    /// Positive = market values team higher than model → increase goose.
    pub signed_edge: f64,
    /// Best single trade for display.
    pub best_trade: Option<Trade>,
}

/// A single profitable trade against the orderbook.
#[derive(Debug, Clone)]
pub struct Trade {
    pub side: Side,
    pub team: String,
    pub round: usize,
    pub price_cents: u32,
    pub model_prob_cents: f64,
    pub edge_cents: f64,
    pub quantity: u32,
    pub ev_dollars: f64,
    pub ticker: String,
}

/// Compute buy and sell edge for a model probability against an orderbook.
///
/// Walks the YES asks: if model_prob > ask_price → buy_edge += (model_prob - ask_price) * qty
/// Walks the YES bids: if model_prob < bid_price → sell_edge += (bid_price - model_prob) * qty
///
/// Returns (buy_edge, sell_edge) in dollar terms (cents converted).
pub fn compute_edge(model_prob: f64, orderbook: &Orderbook) -> (f64, f64) {
    let model_cents = model_prob * 100.0;

    // Buy edge: walk YES asks (ascending price), buy while model > ask
    let mut buy_edge = 0.0;
    for level in &orderbook.yes_asks {
        let ask_cents = level.price as f64;
        if model_cents > ask_cents {
            buy_edge += (model_cents - ask_cents) * level.quantity as f64 / 100.0;
        } else {
            break; // asks are sorted ascending, no more profitable levels
        }
    }

    // Sell edge: walk YES bids (descending price), sell while model < bid
    let mut sell_edge = 0.0;
    for level in &orderbook.yes_bids {
        let bid_cents = level.price as f64;
        if model_cents < bid_cents {
            sell_edge += (bid_cents - model_cents) * level.quantity as f64 / 100.0;
        } else {
            break; // bids are sorted descending, no more profitable levels
        }
    }

    (buy_edge, sell_edge)
}

/// Compute total loss across all markets. Returns (total_edge_dollars, per-market edges).
pub fn compute_total_loss(
    model_probs: &HashMap<(String, usize), f64>,
    orderbooks: &[TeamOrderbook],
) -> (f64, Vec<MarketEdge>) {
    let mut edges = Vec::new();
    let mut total = 0.0;

    for tob in orderbooks {
        let key = (tob.team.clone(), tob.round);
        let model_prob = model_probs.get(&key).copied().unwrap_or(0.0);
        let (buy_edge, sell_edge) = compute_edge(model_prob, &tob.orderbook);
        let total_edge = buy_edge + sell_edge;
        let signed_edge = sell_edge - buy_edge;
        total += total_edge;

        // Find best single trade for display
        let best_trade = find_best_trade(
            model_prob,
            &tob.orderbook,
            &tob.team,
            tob.round,
            &tob.ticker,
        );

        edges.push(MarketEdge {
            team: tob.team.clone(),
            round: tob.round,
            ticker: tob.ticker.clone(),
            model_prob,
            buy_edge,
            sell_edge,
            total_edge,
            signed_edge,
            best_trade,
        });
    }

    (total, edges)
}

/// Find the single most profitable trade level.
fn find_best_trade(
    model_prob: f64,
    orderbook: &Orderbook,
    team: &str,
    round: usize,
    ticker: &str,
) -> Option<Trade> {
    let model_cents = model_prob * 100.0;
    let mut best: Option<Trade> = None;

    for level in &orderbook.yes_asks {
        let ask_cents = level.price as f64;
        if model_cents > ask_cents {
            let edge = model_cents - ask_cents;
            let ev = edge * level.quantity as f64 / 100.0;
            if best.as_ref().is_none_or(|b| ev > b.ev_dollars) {
                best = Some(Trade {
                    side: Side::Buy,
                    team: team.to_string(),
                    round,
                    price_cents: level.price,
                    model_prob_cents: model_cents,
                    edge_cents: edge,
                    quantity: level.quantity,
                    ev_dollars: ev,
                    ticker: ticker.to_string(),
                });
            }
        }
    }

    for level in &orderbook.yes_bids {
        let bid_cents = level.price as f64;
        if model_cents < bid_cents {
            let edge = bid_cents - model_cents;
            let ev = edge * level.quantity as f64 / 100.0;
            if best.as_ref().is_none_or(|b| ev > b.ev_dollars) {
                best = Some(Trade {
                    side: Side::Sell,
                    team: team.to_string(),
                    round,
                    price_cents: level.price,
                    model_prob_cents: model_cents,
                    edge_cents: edge,
                    quantity: level.quantity,
                    ev_dollars: ev,
                    ticker: ticker.to_string(),
                });
            }
        }
    }

    best
}

/// Return the top N trades by EV, sorted descending.
pub fn best_trades(edges: &[MarketEdge], top_n: usize) -> Vec<Trade> {
    let mut trades: Vec<Trade> = edges.iter().filter_map(|e| e.best_trade.clone()).collect();
    trades.sort_by(|a, b| b.ev_dollars.partial_cmp(&a.ev_dollars).unwrap());
    trades.truncate(top_n);
    trades
}

const ROUND_LABELS: [&str; 6] = ["R32", "S16", "E8", "F4", "CG", "CW"];

fn round_label(round: usize) -> &'static str {
    if (1..=6).contains(&round) {
        ROUND_LABELS[round - 1]
    } else {
        "??"
    }
}

/// Print a formatted trade log table.
pub fn print_trade_log(trades: &[Trade]) {
    if trades.is_empty() {
        println!("No profitable trades found.");
        return;
    }

    let max_team = trades
        .iter()
        .map(|t| t.team.len())
        .max()
        .unwrap_or(4)
        .max(4);

    println!();
    println!(
        " {:<4} {:<max_team$}  {:>3}  {:>5}  {:>5}  {:>6}  {:>4}  {:>8}  URL",
        "Side", "Team", "Rnd", "Price", "Model", "Edge", "Qty", "EV($)"
    );
    println!("{}", "-".repeat(max_team + 65));

    for t in trades {
        println!(
            " {:<4} {:<max_team$}  {:>3}  {:>4}\u{00a2}  {:>4.0}\u{00a2}  {:>5.1}\u{00a2}  {:>4}  ${:>7.2}  {}",
            t.side,
            t.team,
            round_label(t.round),
            t.price_cents,
            t.model_prob_cents,
            t.edge_cents,
            t.quantity,
            t.ev_dollars,
            kalshi_url(&t.ticker),
        );
    }
}

/// Print a summary of edge breakdown by round.
pub fn print_edge_summary(edges: &[MarketEdge], total_edge: f64) {
    println!();
    println!("Edge summary:");
    for round in 1..=6 {
        let round_edges: Vec<_> = edges.iter().filter(|e| e.round == round).collect();
        if round_edges.is_empty() {
            continue;
        }
        let round_total: f64 = round_edges.iter().map(|e| e.total_edge).sum();
        let markets_with_edge = round_edges.iter().filter(|e| e.total_edge > 0.001).count();
        println!(
            "  {} : ${:.2} across {} markets ({} with edge)",
            round_label(round),
            round_total,
            round_edges.len(),
            markets_with_edge,
        );
    }
    println!("  Total: ${:.2}", total_edge);
}

/// Build a Kalshi market URL from a ticker.
pub fn kalshi_url(ticker: &str) -> String {
    format!("https://kalshi.com/markets/{}", ticker)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::OrderbookLevel;

    fn make_orderbook(ticker: &str, bids: Vec<(u32, u32)>, asks: Vec<(u32, u32)>) -> Orderbook {
        Orderbook {
            ticker: ticker.to_string(),
            yes_bids: bids
                .into_iter()
                .map(|(p, q)| OrderbookLevel {
                    price: p,
                    quantity: q,
                })
                .collect(),
            yes_asks: asks
                .into_iter()
                .map(|(p, q)| OrderbookLevel {
                    price: p,
                    quantity: q,
                })
                .collect(),
        }
    }

    #[test]
    fn no_edge_when_model_between_spread() {
        // Bid at 40, ask at 60; model at 50 → no edge
        let ob = make_orderbook("TEST", vec![(40, 100)], vec![(60, 100)]);
        let (buy, sell) = compute_edge(0.50, &ob);
        assert_eq!(buy, 0.0);
        assert_eq!(sell, 0.0);
    }

    #[test]
    fn buy_edge_when_model_above_ask() {
        // Ask at 40¢, model at 60% → buy edge = (60 - 40) * 10 / 100 = $2.00
        let ob = make_orderbook("TEST", vec![], vec![(40, 10)]);
        let (buy, sell) = compute_edge(0.60, &ob);
        assert!((buy - 2.0).abs() < 0.001);
        assert_eq!(sell, 0.0);
    }

    #[test]
    fn sell_edge_when_model_below_bid() {
        // Bid at 70¢, model at 50% → sell edge = (70 - 50) * 10 / 100 = $2.00
        let ob = make_orderbook("TEST", vec![(70, 10)], vec![]);
        let (buy, sell) = compute_edge(0.50, &ob);
        assert_eq!(buy, 0.0);
        assert!((sell - 2.0).abs() < 0.001);
    }

    #[test]
    fn multi_level_buy_edge() {
        // Asks at 30¢ (5 qty), 40¢ (10 qty); model at 50%
        // Buy edge: (50-30)*5/100 + (50-40)*10/100 = 1.0 + 1.0 = $2.00
        let ob = make_orderbook("TEST", vec![], vec![(30, 5), (40, 10)]);
        let (buy, sell) = compute_edge(0.50, &ob);
        assert!((buy - 2.0).abs() < 0.001);
        assert_eq!(sell, 0.0);
    }

    #[test]
    fn partial_walk_stops_at_model_price() {
        // Asks at 30¢ (5), 40¢ (10), 60¢ (20); model at 50%
        // Only walks first two levels: (50-30)*5/100 + (50-40)*10/100 = 1.0 + 1.0 = $2.00
        let ob = make_orderbook("TEST", vec![], vec![(30, 5), (40, 10), (60, 20)]);
        let (buy, _) = compute_edge(0.50, &ob);
        assert!((buy - 2.0).abs() < 0.001);
    }

    #[test]
    fn total_loss_aggregates() {
        let tobs = vec![
            TeamOrderbook {
                team: "Duke".to_string(),
                round: 1,
                ticker: "T1".to_string(),
                orderbook: make_orderbook("T1", vec![], vec![(40, 10)]),
            },
            TeamOrderbook {
                team: "UNC".to_string(),
                round: 1,
                ticker: "T2".to_string(),
                orderbook: make_orderbook("T2", vec![(70, 10)], vec![]),
            },
        ];

        let mut model = HashMap::new();
        model.insert(("Duke".to_string(), 1), 0.60); // buy edge: (60-40)*10/100 = $2
        model.insert(("UNC".to_string(), 1), 0.50); // sell edge: (70-50)*10/100 = $2

        let (total, edges) = compute_total_loss(&model, &tobs);
        assert!((total - 4.0).abs() < 0.001);
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn best_trades_sorted_by_ev() {
        let edges = vec![
            MarketEdge {
                team: "A".to_string(),
                round: 1,
                ticker: "T1".to_string(),
                model_prob: 0.5,
                buy_edge: 1.0,
                sell_edge: 0.0,
                total_edge: 1.0,
                signed_edge: -1.0,
                best_trade: Some(Trade {
                    side: Side::Buy,
                    team: "A".to_string(),
                    round: 1,
                    price_cents: 40,
                    model_prob_cents: 50.0,
                    edge_cents: 10.0,
                    quantity: 10,
                    ev_dollars: 1.0,
                    ticker: "T1".to_string(),
                }),
            },
            MarketEdge {
                team: "B".to_string(),
                round: 1,
                ticker: "T2".to_string(),
                model_prob: 0.5,
                buy_edge: 0.0,
                sell_edge: 3.0,
                total_edge: 3.0,
                signed_edge: 3.0,
                best_trade: Some(Trade {
                    side: Side::Sell,
                    team: "B".to_string(),
                    round: 1,
                    price_cents: 70,
                    model_prob_cents: 50.0,
                    edge_cents: 20.0,
                    quantity: 15,
                    ev_dollars: 3.0,
                    ticker: "T2".to_string(),
                }),
            },
        ];

        let top = best_trades(&edges, 1);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].team, "B"); // higher EV
    }

    #[test]
    fn kalshi_url_format() {
        assert_eq!(
            kalshi_url("KXMARMAD-26-DUKE"),
            "https://kalshi.com/markets/KXMARMAD-26-DUKE"
        );
    }
}
