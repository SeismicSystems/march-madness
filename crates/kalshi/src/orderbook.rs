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
        let s = match self {
            Side::Buy => "BUY",
            Side::Sell => "SELL",
        };
        f.pad(s)
    }
}

/// Edge for one team/round market.
///
/// Only one side can have edge in a valid orderbook (bids < asks), so `edge`
/// is a single signed value:
/// - Positive = sell edge (market overvalues team vs model → increase goose)
/// - Negative = buy edge (market undervalues team vs model → decrease goose)
#[derive(Debug, Clone)]
pub struct MarketEdge {
    pub team: String,
    pub round: usize,
    pub ticker: String,
    pub model_prob: f64,
    /// Signed edge in dollars. Positive = sell, negative = buy.
    pub edge: f64,
    /// All profitable trades at each orderbook level.
    pub trades: Vec<Trade>,
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

/// Compute edge for a model probability against an orderbook.
///
/// In a valid orderbook only one side can have edge (bids < asks), so this
/// returns a single signed value:
/// - Negative: model > asks → we'd buy (model undervalued by market)
/// - Positive: model < bids → we'd sell (model overvalued by market)
/// - Zero: model is inside the spread
pub fn compute_edge(model_prob: f64, orderbook: &Orderbook) -> f64 {
    let model_cents = model_prob * 100.0;

    // Buy edge: walk YES asks (ascending price), buy while model > ask
    let mut buy_edge = 0.0;
    for level in &orderbook.yes_asks {
        let ask_cents = level.price as f64;
        if model_cents > ask_cents {
            buy_edge += (model_cents - ask_cents) * level.quantity as f64 / 100.0;
        } else {
            break;
        }
    }

    if buy_edge > 0.0 {
        return -buy_edge;
    }

    // Sell edge: walk YES bids (descending price), sell while model < bid
    let mut sell_edge = 0.0;
    for level in &orderbook.yes_bids {
        let bid_cents = level.price as f64;
        if model_cents < bid_cents {
            sell_edge += (bid_cents - model_cents) * level.quantity as f64 / 100.0;
        } else {
            break;
        }
    }

    sell_edge
}

/// Build all profitable trades for a model probability against an orderbook.
fn find_trades(
    model_prob: f64,
    orderbook: &Orderbook,
    team: &str,
    round: usize,
    ticker: &str,
) -> Vec<Trade> {
    let model_cents = model_prob * 100.0;
    let mut trades = Vec::new();

    for level in &orderbook.yes_asks {
        let ask_cents = level.price as f64;
        if model_cents > ask_cents {
            let edge = model_cents - ask_cents;
            trades.push(Trade {
                side: Side::Buy,
                team: team.to_string(),
                round,
                price_cents: level.price,
                model_prob_cents: model_cents,
                edge_cents: edge,
                quantity: level.quantity,
                ev_dollars: edge * level.quantity as f64 / 100.0,
                ticker: ticker.to_string(),
            });
        } else {
            break;
        }
    }

    for level in &orderbook.yes_bids {
        let bid_cents = level.price as f64;
        if model_cents < bid_cents {
            let edge = bid_cents - model_cents;
            trades.push(Trade {
                side: Side::Sell,
                team: team.to_string(),
                round,
                price_cents: level.price,
                model_prob_cents: model_cents,
                edge_cents: edge,
                quantity: level.quantity,
                ev_dollars: edge * level.quantity as f64 / 100.0,
                ticker: ticker.to_string(),
            });
        } else {
            break;
        }
    }

    trades
}

/// Compute total loss across all markets. Returns (total_abs_edge_dollars, per-market edges).
pub fn compute_total_loss(
    model_probs: &HashMap<(String, usize), f64>,
    orderbooks: &[TeamOrderbook],
) -> (f64, Vec<MarketEdge>) {
    let mut edges = Vec::new();
    let mut total = 0.0;

    for tob in orderbooks {
        let key = (tob.team.clone(), tob.round);
        let model_prob = model_probs.get(&key).copied().unwrap_or(0.0);
        let edge = compute_edge(model_prob, &tob.orderbook);
        total += edge.abs();

        let trades = find_trades(
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
            edge,
            trades,
        });
    }

    (total, edges)
}

/// Collect all profitable trades across all markets, sorted by EV descending.
pub fn all_trades(edges: &[MarketEdge]) -> Vec<Trade> {
    let mut trades: Vec<Trade> = edges.iter().flat_map(|e| e.trades.clone()).collect();
    trades.sort_by(|a, b| b.ev_dollars.partial_cmp(&a.ev_dollars).unwrap());
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

    let rows: Vec<_> = trades
        .iter()
        .map(|t| {
            (
                format!("{}", t.price_cents),
                format!("{:.0}", t.model_prob_cents),
                format!("{:.1}", t.edge_cents),
                format!("{:.2}", t.ev_dollars),
            )
        })
        .collect();

    // Column widths (content only, separators added below).
    let w_side = 4; // "SELL" is longest
    let w_rnd = 3;
    let w_price = rows.iter().map(|r| r.0.len()).max().unwrap_or(5).max(5);
    let w_model = rows.iter().map(|r| r.1.len()).max().unwrap_or(5).max(5);
    let w_edge = rows.iter().map(|r| r.2.len()).max().unwrap_or(4).max(4);
    let w_qty = trades
        .iter()
        .map(|t| (t.quantity.max(1) as f64).log10() as usize + 1)
        .max()
        .unwrap_or(3)
        .max(3);
    let w_ev = rows.iter().map(|r| r.3.len()).max().unwrap_or(5).max(5);

    println!();
    println!(
        " {:<max_team$}  {:>w_side$}  {:>w_rnd$}  {:>w_price$}  {:>w_model$}  {:>w_edge$}  {:>w_qty$}  {:>w_ev$}  URL",
        "Team", "Side", "Rnd", "Price", "Model", "Edge", "Qty", "EV($)"
    );
    let line_width = 1
        + max_team
        + 2
        + w_side
        + 2
        + w_rnd
        + 2
        + w_price
        + 2
        + w_model
        + 2
        + w_edge
        + 2
        + w_qty
        + 2
        + w_ev
        + 2
        + 3;
    println!("{}", "-".repeat(line_width));

    for (t, fmts) in trades.iter().zip(&rows) {
        println!(
            " {:<max_team$}  {:>w_side$}  {:>w_rnd$}  {:>w_price$}  {:>w_model$}  {:>w_edge$}  {:>w_qty$}  {:>w_ev$}  {}",
            t.team,
            t.side,
            round_label(t.round),
            fmts.0,
            fmts.1,
            fmts.2,
            t.quantity,
            fmts.3,
            kalshi_url(&t.ticker),
        );
    }

    let total_ev: f64 = trades.iter().map(|t| t.ev_dollars).sum();
    let total_ev_fmt = format!("{:.2}", total_ev);
    println!("{}", "-".repeat(line_width));
    println!(
        " {:<max_team$}  {:>w_side$}  {:>w_rnd$}  {:>w_price$}  {:>w_model$}  {:>w_edge$}  {:>w_qty$}  {:>w_ev$}",
        "", "", "", "", "", "", "", total_ev_fmt
    );
}

/// Print a summary of edge breakdown by round.
pub fn print_edge_summary(edges: &[MarketEdge], total_edge: f64) {
    // Collect round data so we can right-align the dollar amounts.
    let mut round_data: Vec<(String, f64, usize, usize)> = Vec::new();
    for round in 1..=6 {
        let round_edges: Vec<_> = edges.iter().filter(|e| e.round == round).collect();
        if round_edges.is_empty() {
            continue;
        }
        let round_total: f64 = round_edges.iter().map(|e| e.edge.abs()).sum();
        let markets_with_edge = round_edges.iter().filter(|e| e.edge.abs() > 0.001).count();
        round_data.push((
            round_label(round).to_string(),
            round_total,
            round_edges.len(),
            markets_with_edge,
        ));
    }

    // Determine widths for aligned output.
    let w_amt = round_data
        .iter()
        .map(|(_, v, _, _)| format!("${:.2}", v).len())
        .max()
        .unwrap_or(5)
        .max(format!("${:.2}", total_edge).len());
    let w_markets = round_data
        .iter()
        .map(|(_, _, n, _)| n.to_string().len())
        .max()
        .unwrap_or(1);

    println!();
    println!("Edge summary:");
    for (label, total, num_markets, with_edge) in &round_data {
        let amt = format!("${:.2}", total);
        println!(
            "  {:>3} : {:>w_amt$} across {:>w_markets$} markets ({} with edge)",
            label, amt, num_markets, with_edge
        );
    }
    let total_fmt = format!("${:.2}", total_edge);
    println!("  Tot : {:>w_amt$}", total_fmt);
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
        let ob = make_orderbook("TEST", vec![(40, 100)], vec![(60, 100)]);
        let edge = compute_edge(0.50, &ob);
        assert_eq!(edge, 0.0);
    }

    #[test]
    fn buy_edge_is_negative() {
        // Ask at 40¢, model at 60% → buy edge = -(60-40)*10/100 = -$2.00
        let ob = make_orderbook("TEST", vec![], vec![(40, 10)]);
        let edge = compute_edge(0.60, &ob);
        assert!((edge - -2.0).abs() < 0.001);
    }

    #[test]
    fn sell_edge_is_positive() {
        // Bid at 70¢, model at 50% → sell edge = (70-50)*10/100 = $2.00
        let ob = make_orderbook("TEST", vec![(70, 10)], vec![]);
        let edge = compute_edge(0.50, &ob);
        assert!((edge - 2.0).abs() < 0.001);
    }

    #[test]
    fn multi_level_buy_edge() {
        // Asks at 30¢ (5 qty), 40¢ (10 qty); model at 50%
        let ob = make_orderbook("TEST", vec![], vec![(30, 5), (40, 10)]);
        let edge = compute_edge(0.50, &ob);
        assert!((edge - -2.0).abs() < 0.001);
    }

    #[test]
    fn partial_walk_stops_at_model_price() {
        let ob = make_orderbook("TEST", vec![], vec![(30, 5), (40, 10), (60, 20)]);
        let edge = compute_edge(0.50, &ob);
        assert!((edge - -2.0).abs() < 0.001);
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
        model.insert(("Duke".to_string(), 1), 0.60);
        model.insert(("UNC".to_string(), 1), 0.50);

        let (total, edges) = compute_total_loss(&model, &tobs);
        assert!((total - 4.0).abs() < 0.001);
        assert_eq!(edges.len(), 2);
        assert!(edges[0].edge < 0.0); // Duke: buy
        assert!(edges[1].edge > 0.0); // UNC: sell
    }

    #[test]
    fn all_trades_collects_every_level() {
        let tobs = vec![TeamOrderbook {
            team: "Duke".to_string(),
            round: 1,
            ticker: "T1".to_string(),
            orderbook: make_orderbook("T1", vec![], vec![(30, 5), (40, 10)]),
        }];
        let mut model = HashMap::new();
        model.insert(("Duke".to_string(), 1), 0.50);

        let (_, edges) = compute_total_loss(&model, &tobs);
        let trades = all_trades(&edges);
        assert_eq!(trades.len(), 2); // both ask levels are profitable
        assert!(trades[0].ev_dollars >= trades[1].ev_dollars); // sorted descending
    }

    #[test]
    fn kalshi_url_format() {
        assert_eq!(
            kalshi_url("KXMARMAD-26-DUKE"),
            "https://kalshi.com/markets/KXMARMAD-26-DUKE"
        );
    }
}
