use std::collections::HashMap;
use tracing::{debug, warn};

use crate::team_names::extract_team_name;
use crate::types::{Market, MarketDef};

pub fn parse_f64(s: Option<&str>) -> f64 {
    s.and_then(|v| v.parse().ok()).unwrap_or(0.0)
}

/// Parsed NBBO for a single market.
pub struct NbboProb {
    pub has_bid: bool,
    pub fair_value: f64,
    pub ask: f64,
}

/// Minimum bid size (in contracts) to count as a real bid.
pub const MIN_BID_SIZE: f64 = 10.0;

/// Compute fair value from NBBO using microprice (book pressure).
///
/// microprice = (bid * ask_size + ask * bid_size) / (bid_size + ask_size)
///
/// Falls back to midpoint if size data is missing.
pub fn compute_fair_value(market: &Market) -> NbboProb {
    let bid = parse_f64(market.yes_bid_dollars.as_deref());
    let ask = parse_f64(market.yes_ask_dollars.as_deref());
    let bid_size = parse_f64(market.yes_bid_size_fp.as_deref());
    let ask_size = parse_f64(market.yes_ask_size_fp.as_deref());

    let has_bid = bid > 0.0 && bid_size >= MIN_BID_SIZE;

    let fair_value = if !has_bid {
        0.0
    } else if bid_size > 0.0 && ask_size > 0.0 && ask > bid {
        (bid * ask_size + ask * bid_size) / (bid_size + ask_size)
    } else if ask > 0.0 && ask >= bid {
        (bid + ask) / 2.0
    } else {
        bid
    };

    NbboProb {
        has_bid,
        fair_value,
        ask,
    }
}

/// Compute fair value from NBBO only (bid/ask, no size info).
/// Falls back to midpoint; no bid -> 0.
pub fn compute_fair_value_nbbo(bid: f64, ask: f64) -> NbboProb {
    let has_bid = bid > 0.0;

    let fair_value = if !has_bid {
        0.0
    } else if ask > bid {
        (bid + ask) / 2.0
    } else {
        bid
    };

    NbboProb {
        has_bid,
        fair_value,
        ask,
    }
}

/// Normalize probabilities for a round.
pub fn normalize_round(
    markets: &[Market],
    mdef: &MarketDef,
    name_map: &HashMap<String, String>,
    raw_mode: bool,
) -> Vec<(String, f64)> {
    let mut bid_teams: Vec<(String, f64)> = Vec::new();
    let mut no_bid_teams: Vec<(String, f64)> = Vec::new();

    for m in markets {
        let ob = compute_fair_value(m);
        let raw_name = extract_team_name(m);
        let name = name_map.get(&raw_name).cloned().unwrap_or(raw_name);

        if ob.has_bid {
            bid_teams.push((name, ob.fair_value));
        } else {
            no_bid_teams.push((name, ob.ask));
        }
    }

    normalize_teams(bid_teams, no_bid_teams, mdef, raw_mode)
}

/// Normalize from pre-split bid/no-bid teams (shared by REST and WS paths).
pub fn normalize_teams(
    mut bid_teams: Vec<(String, f64)>,
    no_bid_teams: Vec<(String, f64)>,
    mdef: &MarketDef,
    raw_mode: bool,
) -> Vec<(String, f64)> {
    for (name, ask) in &no_bid_teams {
        if *ask > 0.10 {
            warn!(
                "{} has NO BID but ask={:.0}% — may deserve a bid",
                name,
                ask * 100.0
            );
        }
    }

    if raw_mode {
        let mut result: Vec<(String, f64)> = bid_teams;
        for (name, _) in &no_bid_teams {
            result.push((name.clone(), 0.0));
        }
        return result;
    }

    let bid_sum: f64 = bid_teams.iter().map(|(_, p)| *p).sum();
    bid_teams.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    let mut result: Vec<(String, f64)> = Vec::new();

    if bid_sum >= mdef.expected_sum {
        let scale = mdef.expected_sum / bid_sum;
        debug!(
            "{}: {} bid markets (sum={:.4}), {} no-bid -> normalizing down (scale={:.4})",
            mdef.label,
            bid_teams.len(),
            bid_sum,
            no_bid_teams.len(),
            scale
        );
        for (name, p) in &bid_teams {
            result.push((name.clone(), (p * scale).min(1.0)));
        }
    } else {
        let remainder = mdef.expected_sum - bid_sum;
        let per_no_bid = if no_bid_teams.is_empty() {
            0.0
        } else {
            remainder / no_bid_teams.len() as f64
        };
        debug!(
            "{}: {} bid markets (sum={:.4}), {} no-bid -> distributing {:.4} remainder ({:.6} each)",
            mdef.label,
            bid_teams.len(),
            bid_sum,
            no_bid_teams.len(),
            remainder,
            per_no_bid
        );
        for (name, p) in &bid_teams {
            result.push((name.clone(), p.min(1.0)));
        }
        for (name, _) in &no_bid_teams {
            result.push((name.clone(), per_no_bid));
        }
    }

    result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let max_name = result
        .iter()
        .take(5)
        .map(|(n, _)| n.len())
        .max()
        .unwrap_or(0);
    for (name, p) in result.iter().take(5) {
        debug!(
            "{:<width$}  {:>2}%",
            name,
            (p * 100.0).round() as i32,
            width = max_name
        );
    }

    result
}
