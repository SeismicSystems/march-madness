/**
 * ESPN team ID mapping for NCAA tournament team logos.
 * Logo URL pattern: https://a.espncdn.com/i/teamlogos/ncaa/500/{id}.png
 *
 * Keys must match team names exactly as they appear in tournament.json.
 * First Four combo names (e.g. "Texas/NC State") are intentionally omitted.
 */
const ESPN_TEAM_IDS: Record<string, number> = {
  Akron: 2006,
  Alabama: 333,
  Arizona: 12,
  Arkansas: 8,
  BYU: 252,
  "Cal Baptist": 2856,
  Clemson: 228,
  Duke: 150,
  Florida: 57,
  Furman: 231,
  Georgia: 61,
  Gonzaga: 2250,
  Hawaii: 62,
  "High Point": 2272,
  Hofstra: 2275,
  Houston: 248,
  Idaho: 70,
  Illinois: 356,
  Iowa: 2294,
  "Iowa St.": 66,
  Kansas: 2305,
  "Kennesaw St.": 338,
  Kentucky: 96,
  "Long Island": 2344,
  Louisville: 97,
  McNeese: 2377,
  "Miami (FL)": 2390,
  Michigan: 130,
  "Michigan St.": 127,
  Missouri: 142,
  Nebraska: 158,
  "North Carolina": 153,
  "North Dakota St.": 2449,
  "Northern Iowa": 2460,
  "Ohio St.": 194,
  Penn: 219,
  Purdue: 2509,
  "Queens (N.C.)": 3101,
  "Saint Louis": 139,
  "Saint Mary's": 2608,
  "Santa Clara": 2541,
  Siena: 2547,
  "South Florida": 58,
  "St. John's": 2599,
  TCU: 2628,
  Tennessee: 2633,
  "Tennessee St.": 2634,
  "Texas A&M": 245,
  "Texas Tech": 2641,
  Troy: 2653,
  UCF: 2116,
  UCLA: 26,
  UConn: 41,
  "Utah St.": 328,
  Vanderbilt: 238,
  VCU: 2670,
  Villanova: 222,
  Virginia: 258,
  Wisconsin: 275,
  "Wright St.": 2750,
  "Prairie View A&M": 2504,
  Lehigh: 2329,
  Texas: 251,
  "NC State": 152,
  SMU: 2567,
  UMBC: 2692,
  Howard: 47,
};

/**
 * ESPN shortDisplayName abbreviations for teams whose tournament.json
 * name exceeds 10 characters. Only long names get abbreviated — short
 * names are already readable at bracket scale.
 */
const ESPN_ABBREVIATIONS: Record<string, string> = {
  "Cal Baptist": "CBU",
  "Kennesaw St.": "Kennesaw",
  "Long Island": "LIU",
  "Michigan St.": "Mich. St.",
  "North Carolina": "UNC",
  "North Dakota St.": "NDSU",
  "Northern Iowa": "UNI",
  "Queens (N.C.)": "Queens",
  "Saint Louis": "SLU",
  "Saint Mary's": "St. Mary's",
  "Santa Clara": "S. Clara",
  "South Florida": "USF",
  "Tennessee St.": "Tenn. St.",
  "Texas Tech": "Tex. Tech",
  "Miami (FL)": "Miami FL",
  "High Point": "High Pt.",
  "Wright St.": "Wright",
  "Tenn. St.": "Tenn",

  // First Four combo names
  "Miami (Ohio)/SMU": "M-OH/SMU",
  "Prairie View A&M/Lehigh": "PVAMU/LEH",
  "Texas/NC State": "TEX/NCST",
  "UMBC/Howard": "UMBC/HOW",
};

/**
 * Returns an ESPN shortDisplayName abbreviation for teams with names
 * longer than 10 characters. Returns null for short names (already readable).
 */
export function getTeamAbbreviation(teamName: string): string | null {
  if (teamName.length <= 9) return null;
  return ESPN_ABBREVIATIONS[teamName] ?? null;
}

export function getTeamLogoUrl(teamName: string): string | null {
  const id = ESPN_TEAM_IDS[teamName];
  if (id === undefined) return null;
  return `https://a.espncdn.com/i/teamlogos/ncaa/500/${id}.png`;
}
