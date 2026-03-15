#!/usr/bin/env python3
"""
Fit logistic regression: KenPom stats -> P(advance to round R).

Uses raw (non-normalized) Kalshi market probabilities as training labels.
No-bid markets are assigned DEFAULT_NO_BID_PROB.

Features: polynomial expansion (degree 2) of net_rtg, ortg, drtg, pace —
includes all squares and pairwise interactions. Regularized (C=0.1) to
prevent overfitting with small sample sizes.

Outputs:
  - data/{YEAR}/kenpom_anchor_model.json: model coefficients & anchor ranges
  - data/{YEAR}/plots/round_N_*.png: fit quality plots (gitignored)

Usage:
    # First generate raw Kalshi data:
    cargo run --bin kalshi -- --raw
    # Then fit the model:
    python scripts/fit_kenpom_model.py
"""

import csv
import json
import sys
from pathlib import Path

import numpy as np

try:
    import matplotlib
    matplotlib.use("Agg")  # non-interactive backend
    import matplotlib.pyplot as plt
    HAS_MATPLOTLIB = True
except ImportError:
    HAS_MATPLOTLIB = False
    print("Warning: matplotlib not installed, skipping plot generation")

from sklearn.linear_model import LogisticRegression
from sklearn.preprocessing import PolynomialFeatures, StandardScaler

# No-bid teams get this probability as a training label
DEFAULT_NO_BID_PROB = 0.001

# How many standard deviations wide the anchor range should be.
ANCHOR_WIDTH_SIGMA = 2.0

# Pseudo-sample size per team for discretizing probabilities into binary labels.
PSEUDO_SAMPLE_SIZE = 1000

# Regularization strength (lower = more regularization).
# Needed to prevent overfitting with polynomial features on small samples.
REGULARIZATION_C = 0.1

# Polynomial degree for feature expansion.
POLY_DEGREE = 2

ROUND_LABELS = {1: "R32", 2: "S16", 3: "E8", 4: "F4", 5: "ChampGame", 6: "Champion"}
EXPECTED_SUMS = {1: 32, 2: 16, 3: 8, 4: 4, 5: 2, 6: 1}

DATA_DIR = Path(__file__).resolve().parent.parent / "data"
YEAR = 2026

# Base feature names before polynomial expansion
BASE_FEATURE_NAMES = ["net_rtg", "ortg", "drtg", "pace"]


def load_kenpom(teams_csv: Path) -> dict[str, dict]:
    """Load KenPom data, return dict of team -> {ortg, drtg, pace, net_rtg}."""
    teams = {}
    with open(teams_csv) as f:
        reader = csv.DictReader(f)
        for row in reader:
            name = row["team"]
            ortg = float(row["ortg"])
            drtg = float(row["drtg"])
            pace = float(row["pace"])
            teams[name] = {
                "ortg": ortg,
                "drtg": drtg,
                "pace": pace,
                "net_rtg": ortg - drtg,
            }
    return teams


def load_raw_kalshi(raw_csv: Path) -> dict[int, list[tuple[str, float]]]:
    """Load raw Kalshi probabilities, return dict of round -> [(team, prob)]."""
    by_round: dict[int, list[tuple[str, float]]] = {}
    with open(raw_csv) as f:
        reader = csv.DictReader(f)
        for row in reader:
            team = row["team"]
            rnd = int(row["round"])
            prob = float(row["probability"])
            by_round.setdefault(rnd, []).append((team, prob))
    return by_round


def build_base_features(kenpom_stats: dict) -> np.ndarray:
    """Build base feature vector: [net_rtg, ortg, drtg, pace]."""
    return np.array([
        kenpom_stats["net_rtg"],
        kenpom_stats["ortg"],
        kenpom_stats["drtg"],
        kenpom_stats["pace"],
    ])


def fit_models(kenpom: dict, kalshi_by_round: dict, plots_dir: Path = None) -> dict:
    """
    Fit regularized logistic regression per round with polynomial features.

    Base features: net_rtg, ortg, drtg, pace
    Expanded: degree-2 polynomial (squares + interactions) → ~14 features
    Regularized with C=0.1 to prevent overfitting on small samples.

    Uses sample_weight to approximate continuous probability targets.
    """
    models = {}

    for rnd in sorted(EXPECTED_SUMS.keys()):
        label = ROUND_LABELS[rnd]

        # Build training data for this round
        team_names = []
        X_list = []
        y_list = []

        for team, prob in kalshi_by_round.get(rnd, []):
            if team not in kenpom:
                continue
            stats = kenpom[team]
            if prob <= 0.0:
                prob = DEFAULT_NO_BID_PROB
            prob = max(DEFAULT_NO_BID_PROB, min(prob, 1.0 - DEFAULT_NO_BID_PROB))
            team_names.append(team)
            X_list.append(build_base_features(stats))
            y_list.append(prob)

        if len(X_list) < 5:
            print(f"  Round {rnd} ({label}): too few data points ({len(X_list)}), skipping")
            continue

        X_base = np.array(X_list)
        y = np.array(y_list)

        # Polynomial feature expansion + standardization
        poly = PolynomialFeatures(degree=POLY_DEGREE, include_bias=False)
        X_poly = poly.fit_transform(X_base)
        feature_names = poly.get_feature_names_out(BASE_FEATURE_NAMES).tolist()

        scaler = StandardScaler()
        X_scaled = scaler.fit_transform(X_poly)

        # Use sample_weight to approximate continuous probability targets
        N = PSEUDO_SAMPLE_SIZE
        X_doubled = np.vstack([X_scaled, X_scaled])
        y_binary = np.array([1] * len(X_scaled) + [0] * len(X_scaled))
        weights = np.concatenate([y * N, (1 - y) * N])

        clf = LogisticRegression(max_iter=10000, solver="lbfgs", C=REGULARIZATION_C)
        clf.fit(X_doubled, y_binary, sample_weight=weights)

        # Compute predictions and cross-entropy (log loss)
        y_pred = clf.predict_proba(X_scaled)[:, 1]
        eps = 1e-15
        y_clip = np.clip(y_pred, eps, 1 - eps)
        log_loss = float(-np.mean(y * np.log(y_clip) + (1 - y) * np.log(1 - y_clip)))
        mae = float(np.mean(np.abs(y - y_pred)))

        model_info = {
            "round": rnd,
            "label": label,
            "n_teams": len(X_list),
            "log_loss": round(log_loss, 6),
            "mae": round(mae, 6),
            "anchor_half_width": round(ANCHOR_WIDTH_SIGMA * mae, 6),
            "poly_degree": POLY_DEGREE,
            "feature_names": feature_names,
            "intercept": round(float(clf.intercept_[0]), 8),
            "coefs": [round(float(c), 8) for c in clf.coef_[0]],
            "scaler_mean": [round(float(m), 8) for m in scaler.mean_],
            "scaler_scale": [round(float(s), 8) for s in scaler.scale_],
        }

        models[rnd] = model_info

        print(f"  Round {rnd} ({label}): n={len(X_list)}, {len(feature_names)} features, "
              f"C={REGULARIZATION_C}, log_loss={log_loss:.4f}, mae={mae:.4f}")

        # Show top 3 predictions vs actuals
        order = np.argsort(-y)
        for idx in order[:3]:
            net_rtg = kenpom[team_names[idx]]["net_rtg"]
            print(f"    {team_names[idx]}: kalshi={y[idx]:.4f}, model={y_pred[idx]:.4f}, "
                  f"net_rtg={net_rtg:.1f}")

        # Generate plot
        if HAS_MATPLOTLIB and plots_dir:
            plot_round(rnd, label, team_names, kenpom, y, y_pred, log_loss, plots_dir)

    return models


def plot_round(rnd: int, label: str, team_names: list, kenpom: dict,
               y_actual: np.ndarray, y_pred: np.ndarray, log_loss: float,
               plots_dir: Path = None):
    """Generate fit quality plot for a single round."""
    plots_dir.mkdir(parents=True, exist_ok=True)

    net_rtgs = np.array([kenpom[t]["net_rtg"] for t in team_names])

    fig, axes = plt.subplots(1, 2, figsize=(14, 6))

    # Left: net_rtg vs probability (actual and predicted)
    ax = axes[0]
    ax.scatter(net_rtgs, y_actual, alpha=0.6, s=25, label="Kalshi (raw)", zorder=3)
    ax.scatter(net_rtgs, y_pred, alpha=0.6, s=25, marker="x", color="red",
               label="Model prediction", zorder=3)

    # Draw residual lines
    for i in range(len(net_rtgs)):
        ax.plot([net_rtgs[i], net_rtgs[i]], [y_actual[i], y_pred[i]],
                color="gray", alpha=0.3, linewidth=0.8)

    # Label top teams
    top_idx = np.argsort(-y_actual)[:8]
    for idx in top_idx:
        ax.annotate(team_names[idx], (net_rtgs[idx], y_actual[idx]),
                    fontsize=7, alpha=0.7,
                    xytext=(5, 5), textcoords="offset points")

    ax.set_xlabel("KenPom Net Rating (AdjO - AdjD)")
    ax.set_ylabel("P(advance)")
    ax.set_title(f"Round {rnd}: {label} — Net Rating vs Probability")
    ax.legend(fontsize=8)
    ax.grid(True, alpha=0.3)

    # Right: actual vs predicted (calibration plot)
    ax = axes[1]
    ax.scatter(y_pred, y_actual, alpha=0.6, s=25, zorder=3)
    lims = [0, max(y_actual.max(), y_pred.max()) * 1.1]
    ax.plot(lims, lims, "k--", alpha=0.3, label="Perfect calibration")

    # Label outliers
    abs_residuals = np.abs(y_actual - y_pred)
    outlier_idx = np.argsort(-abs_residuals)[:5]
    for idx in outlier_idx:
        ax.annotate(team_names[idx], (y_pred[idx], y_actual[idx]),
                    fontsize=7, alpha=0.7,
                    xytext=(5, 5), textcoords="offset points")

    ax.set_xlabel("Model Predicted P(advance)")
    ax.set_ylabel("Kalshi Raw P(advance)")
    ax.set_title(f"Round {rnd}: {label} — Calibration (log_loss={log_loss:.4f})")
    ax.legend(fontsize=8)
    ax.grid(True, alpha=0.3)

    plt.tight_layout()
    out_path = plots_dir / f"round_{rnd}_{label}.png"
    plt.savefig(out_path, dpi=150)
    plt.close()
    print(f"    Plot saved: {out_path}")


def predict_prob(model_info: dict, kenpom_stats: dict) -> float:
    """Predict P(advance) from saved model coefficients and KenPom stats."""
    base = build_base_features(kenpom_stats)
    poly = PolynomialFeatures(degree=model_info["poly_degree"], include_bias=False)
    # fit_transform needs 2D input
    X_poly = poly.fit_transform(base.reshape(1, -1))[0]
    # Standardize using saved scaler params
    mean = np.array(model_info["scaler_mean"])
    scale = np.array(model_info["scaler_scale"])
    X_scaled = (X_poly - mean) / scale
    # Compute logit
    logit = model_info["intercept"] + np.dot(model_info["coefs"], X_scaled)
    return 1.0 / (1.0 + np.exp(-logit))


def main():
    season_dir = DATA_DIR / str(YEAR)
    teams_csv = season_dir / "kenpom.csv"
    raw_csv = season_dir / "targets_kalshi_raw.csv"
    output_json = season_dir / "kenpom_anchor_model.json"

    if not raw_csv.exists():
        print(f"Error: {raw_csv} not found.")
        print("Generate it first:")
        print(f"  cargo run --bin kalshi -- --raw")
        sys.exit(1)

    if not teams_csv.exists():
        print(f"Error: {teams_csv} not found.")
        sys.exit(1)

    print("Loading KenPom data...")
    kenpom = load_kenpom(teams_csv)
    print(f"  {len(kenpom)} teams")

    print("Loading raw Kalshi data...")
    kalshi_by_round = load_raw_kalshi(raw_csv)
    total_rows = sum(len(v) for v in kalshi_by_round.values())
    print(f"  {total_rows} rows across {len(kalshi_by_round)} rounds")

    plots_dir = season_dir / "plots"

    print(f"\nFitting logistic models per round "
          f"(degree-{POLY_DEGREE} polynomial, C={REGULARIZATION_C}):")
    models = fit_models(kenpom, kalshi_by_round, plots_dir)

    # Save model coefficients
    output = {
        "description": "KenPom stats -> P(advance) logistic model per round",
        "base_features": BASE_FEATURE_NAMES,
        "poly_degree": POLY_DEGREE,
        "regularization_C": REGULARIZATION_C,
        "anchor_width_sigma": ANCHOR_WIDTH_SIGMA,
        "default_no_bid_prob": DEFAULT_NO_BID_PROB,
        "rounds": {str(k): v for k, v in models.items()},
    }

    with open(output_json, "w") as f:
        json.dump(output, f, indent=2)
    print(f"\nModel saved to {output_json}")

    # Print anchor range examples
    print("\nAnchor range examples (for selected teams):")
    example_teams = ["Duke", "Florida", "Houston", "Auburn", "Iowa St.", "Wofford"]
    for team in example_teams:
        if team not in kenpom:
            continue
        stats = kenpom[team]
        print(f"\n  {team} (net_rtg={stats['net_rtg']:.1f}, "
              f"ortg={stats['ortg']:.1f}, drtg={stats['drtg']:.1f}, pace={stats['pace']:.1f}):")
        for rnd, m in sorted(models.items()):
            pred = predict_prob(m, stats)
            hw = m["anchor_half_width"]
            lo = max(0.0, pred - hw)
            hi = min(1.0, pred + hw)
            print(f"    R{rnd} ({m['label']}): pred={pred:.4f}, range=[{lo:.4f}, {hi:.4f}]")


if __name__ == "__main__":
    main()
