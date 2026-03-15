use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Metrics {
    pub ortg: f64,
    pub drtg: f64,
    pub pace: f64,
}

impl Metrics {
    pub fn flip(&self) -> Metrics {
        Metrics {
            ortg: self.drtg,
            drtg: self.ortg,
            pace: self.pace,
        }
    }
}
