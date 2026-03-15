#!/usr/bin/env python3
"""
Scrape KenPom ratings and output KenPom CSV for bracket calibration.

Uses kenpompy + cloudscraper to bypass Cloudflare protection.
The homepage (Pomeroy ratings) is free — no subscription needed.

Outputs:
    data/{YEAR}/kenpom.csv  (team, ortg, drtg, pace)

Seeds/regions are stored separately in data/{YEAR}/bracket.csv (maintained
manually or from another source). The Rust loader joins both files and
validates that every bracket team has a KenPom entry.

Usage:
    python scripts/scrape_kenpom.py
"""

import argparse
import csv
import sys

from pathlib import Path

try:
    import cloudscraper
    import kenpompy.misc as kpm
except ImportError:
    print("Install dependencies: uv pip install kenpompy cloudscraper")
    sys.exit(1)

DATA_DIR = Path(__file__).resolve().parent.parent / "data"
YEAR = 2026


def fetch_kenpom(year: int) -> list[dict]:
    """Fetch KenPom ratings via kenpompy (handles Cloudflare)."""
    browser = cloudscraper.create_scraper()
    print(f"Fetching KenPom ratings for {year}...")
    df = kpm.get_pomeroy_ratings(browser, season=str(year))
    print(f"  Got {len(df)} teams")

    teams = []
    for _, row in df.iterrows():
        team_name = str(row["Team"]).strip()
        if not team_name:
            continue
        try:
            ortg = float(row["AdjO"])
            drtg = float(row["AdjD"])
            pace = float(row["AdjT"])
        except (ValueError, KeyError):
            continue
        teams.append({
            "team": team_name,
            "ortg": ortg,
            "drtg": drtg,
            "pace": pace,
        })
    return teams


def load_bracket_teams(seed_file: Path) -> dict[str, tuple[int, str]]:
    """Load seed/region data from a bracket CSV (team,seed,region)."""
    seeds = {}
    if not seed_file.exists():
        return seeds
    with open(seed_file) as f:
        reader = csv.DictReader(f)
        for row in reader:
            if "seed" in row and "region" in row:
                try:
                    seeds[row["team"]] = (int(row["seed"]), row["region"])
                except (ValueError, KeyError):
                    pass
    return seeds


def main():
    parser = argparse.ArgumentParser(description="Scrape KenPom ratings")
    parser.add_argument(
        "--seeds-from",
        type=Path,
        help="Load bracket teams from this CSV (default: data/{YEAR}/bracket.csv)",
    )
    parser.add_argument(
        "--bracket-only",
        action="store_true",
        help="Only output teams that have seeds (tournament teams only)",
    )
    args = parser.parse_args()

    teams = fetch_kenpom(YEAR)

    season_dir = DATA_DIR / str(YEAR)
    season_dir.mkdir(parents=True, exist_ok=True)

    # Load bracket teams for filtering (if --bracket-only)
    seed_file = args.seeds_from or season_dir / "bracket.csv"
    seeds = load_bracket_teams(seed_file)
    if seeds:
        print(f"Loaded {len(seeds)} bracket teams from {seed_file}")

    # Write KenPom ratings file (team, ortg, drtg, pace)
    kenpom_path = season_dir / "kenpom.csv"

    count = 0
    with open(kenpom_path, "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["team", "ortg", "drtg", "pace"])
        for t in teams:
            if args.bracket_only:
                if t["team"] not in seeds:
                    continue
            writer.writerow([t["team"], t["ortg"], t["drtg"], t["pace"]])
            count += 1

    print(f"Wrote {count} teams to {kenpom_path}")

    # Report any tournament teams not found in KenPom
    if seeds:
        kenpom_names = {t["team"] for t in teams}
        missing = set(seeds.keys()) - kenpom_names
        if missing:
            print(f"\nWARNING: {len(missing)} tournament teams not found in KenPom:")
            for name in sorted(missing):
                print(f"  - {name}")


if __name__ == "__main__":
    main()
