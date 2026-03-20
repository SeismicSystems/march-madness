pub const AVERAGE_PACE: f64 = 68.0;
pub const AVERAGE_RATING: f64 = 105.0;

pub const MAX_PACE: f64 = 80.0;
pub const MIN_PACE: f64 = 55.0;

pub const MAX_RTG: f64 = 135.0;
pub const MIN_RTG: f64 = 75.0;

/// Default KenPom-style Bayesian postgame metric adjustment factor.
pub const DEFAULT_KENPOM_UPDATE_FACTOR: f64 = 0.05;

/// Default pace dispersion ratio (variance / mean) for possession count sampling.
/// - d < 1: underdispersed (binomial) — tighter than Poisson
/// - d = 1: Poisson
/// - d > 1: overdispersed (negative binomial) — wider than Poisson
///
/// Calibrated to d=0.3 via score-dist sweep against NCAA tournament empirical
/// targets (~142 avg total, ~19 total stddev, ~6% OT). At d=0.3 total stddev ≈ 20,
/// closest to the empirical ~19. See issue #48 for further calibration notes.
pub const DEFAULT_PACE_D: f64 = 0.3;
