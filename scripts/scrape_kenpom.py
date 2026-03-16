#!/usr/bin/env python3
"""
Scrape KenPom ratings and output KenPom CSV for bracket calibration.

Uses kenpompy + cloudscraper to bypass Cloudflare protection.
The homepage (Pomeroy ratings) is free — no subscription needed.

Outputs:
    data/{YEAR}/kenpom.csv  (team, ortg, drtg, pace)

Team names are mapped to NCAA canonical names via data/mappings.toml.
When --bracket-only is used, filters to teams in data/{YEAR}/tournament.json.

For First Four slots (e.g. "Texas/NC State"), both individual teams are written
to kenpom.csv. The bracket-sim loader handles averaging them at load time.

Usage:
    python scripts/scrape_kenpom.py
    python scripts/scrape_kenpom.py --bracket-only
"""

import argparse
import csv
import json
import sys

from pathlib import Path

try:
    import cloudscraper
    import kenpompy.misc as kpm
except ImportError:
    print("Install dependencies: uv pip install kenpompy cloudscraper")
    sys.exit(1)

try:
    import tomllib
except ImportError:
    try:
        import tomli as tomllib
    except ImportError:
        print("Python 3.11+ required (for tomllib), or: uv pip install tomli")
        sys.exit(1)

DATA_DIR = Path(__file__).resolve().parent.parent / "data"
YEAR = 2026


def load_name_mappings() -> dict[str, str]:
    """Load kenpom → NCAA name mappings from data/mappings.toml."""
    mappings_path = DATA_DIR / "mappings.toml"
    if not mappings_path.exists():
        return {}
    with open(mappings_path, "rb") as f:
        config = tomllib.load(f)
    return config.get("kenpom", {})


def load_tournament_teams(tournament_path: Path) -> set[str]:
    """Load team names from tournament.json, expanding First Four entries."""
    if not tournament_path.exists():
        return set()
    with open(tournament_path) as f:
        data = json.load(f)
    names = set()
    for team in data.get("teams", []):
        first_four = team.get("firstFour")
        if first_four:
            # Add both individual First Four team names
            for ff_name in first_four:
                names.add(ff_name)
        else:
            names.add(team["name"])
    return names


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


def main():
    parser = argparse.ArgumentParser(description="Scrape KenPom ratings")
    parser.add_argument(
        "--bracket-only",
        action="store_true",
        help="Only output teams in the tournament bracket (from tournament.json)",
    )
    parser.add_argument(
        "--year",
        type=int,
        default=YEAR,
        help=f"Tournament year (default: {YEAR})",
    )
    args = parser.parse_args()

    name_map = load_name_mappings()
    teams = fetch_kenpom(args.year)

    # Apply name mappings (kenpom → NCAA canonical)
    for t in teams:
        if t["team"] in name_map:
            t["team"] = name_map[t["team"]]

    season_dir = DATA_DIR / str(args.year) / "men"
    season_dir.mkdir(parents=True, exist_ok=True)

    # Load bracket teams for filtering (if --bracket-only)
    bracket_teams: set[str] = set()
    if args.bracket_only:
        tournament_path = season_dir / "tournament.json"
        bracket_teams = load_tournament_teams(tournament_path)
        if bracket_teams:
            print(f"Loaded {len(bracket_teams)} bracket teams from {tournament_path}")
        else:
            print(f"WARNING: no teams found in {tournament_path}")

    # Filter to bracket teams if requested
    if args.bracket_only:
        teams = [t for t in teams if t["team"] in bracket_teams]

    # Sort by net rating (ortg - drtg), best first
    teams.sort(key=lambda t: t["drtg"] - t["ortg"])

    # Write KenPom ratings file (team, ortg, drtg, pace)
    kenpom_path = season_dir / "kenpom.csv"

    with open(kenpom_path, "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["team", "ortg", "drtg", "pace"])
        for t in teams:
            writer.writerow([t["team"], t["ortg"], t["drtg"], t["pace"]])
    count = len(teams)

    print(f"Wrote {count} teams to {kenpom_path}")

    # Report any tournament teams not found in KenPom
    if bracket_teams:
        kenpom_names = {t["team"] for t in teams}
        missing = bracket_teams - kenpom_names
        if missing:
            print(f"\nWARNING: {len(missing)} tournament teams not found in KenPom:")
            for name in sorted(missing):
                print(f"  - {name}")


if __name__ == "__main__":
    main()
