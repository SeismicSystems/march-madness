// Bracket encoding/decoding — converts between game picks and bytes8 hex.

export interface BracketGame {
  round: number;
  game: number;
  winner: string;
}

export interface DecodedBracket {
  champion: string;
  runnerUp: string;
  finalFour: string[];
  eliteEight: string[];
  sweetSixteen: string[];
  roundOf32: string[];
  games: BracketGame[];
}

/**
 * Encode a list of 63 game winners into a bytes8 hex string.
 * Bit 63 (MSB) = sentinel (always 1), bits 62-0 = game outcomes.
 * @param picks - Array of 63 booleans (true = team1/higher seed wins)
 * @returns 0x-prefixed 16-char hex string
 */
export function encodeBracket(picks: boolean[]): `0x${string}` {
  if (picks.length !== 63) {
    throw new Error(`Expected 63 picks, got ${picks.length}`);
  }

  let bits = BigInt(0);
  // MSB sentinel
  bits |= BigInt(1) << BigInt(63);

  // Game outcomes: bit 62 = first game, bit 0 = championship
  for (let i = 0; i < 63; i++) {
    if (picks[i]) {
      bits |= BigInt(1) << BigInt(62 - i);
    }
  }

  return `0x${bits.toString(16).padStart(16, "0")}` as `0x${string}`;
}

/**
 * Decode a bytes8 hex string into structured bracket data.
 * @param hex - 0x-prefixed 16-char hex string
 * @param teams - Array of 64 team names in bracket order
 * @returns Decoded bracket with game results
 */
export function decodeBracket(
  hex: `0x${string}`,
  teams: string[],
): DecodedBracket {
  if (teams.length !== 64) {
    throw new Error(`Expected 64 teams, got ${teams.length}`);
  }

  const bits = BigInt(hex);
  const picks: boolean[] = [];

  for (let i = 0; i < 63; i++) {
    picks.push(((bits >> BigInt(62 - i)) & BigInt(1)) === BigInt(1));
  }

  // Simulate tournament
  let currentTeams = [...teams];
  const games: BracketGame[] = [];
  let round = 0;
  let pickIdx = 0;

  while (currentTeams.length > 1) {
    const nextRound: string[] = [];
    for (let g = 0; g < currentTeams.length; g += 2) {
      const team1 = currentTeams[g];
      const team2 = currentTeams[g + 1];
      const winner = picks[pickIdx] ? team1 : team2;
      games.push({ round, game: g / 2, winner });
      nextRound.push(winner);
      pickIdx++;
    }
    currentTeams = nextRound;
    round++;
  }

  const champion = currentTeams[0];

  // The runner-up is the loser of the championship game.
  // The championship game's two participants are the winners of the two Final Four games.
  // The Final Four games are at round 4, games 0 and 1.
  const finalFourWinners = games.filter((g) => g.round === 4).map((g) => g.winner);
  const runnerUp = finalFourWinners.find((t) => t !== champion) ?? "";

  // Extract by round from games
  const finalFourGames = games.filter((g) => g.round === 4);
  const eliteEightGames = games.filter((g) => g.round === 3);
  const sweetSixteenGames = games.filter((g) => g.round === 2);
  const roundOf32Games = games.filter((g) => g.round === 1);

  return {
    champion,
    runnerUp,
    finalFour: finalFourGames.map((g) => g.winner),
    eliteEight: eliteEightGames.map((g) => g.winner),
    sweetSixteen: sweetSixteenGames.map((g) => g.winner),
    roundOf32: roundOf32Games.map((g) => g.winner),
    games,
  };
}

/**
 * Validate a bytes8 bracket hex string.
 * Checks:
 * - Correct format: 0x-prefixed, 16 hex characters (18 chars total)
 * - Sentinel bit: MSB (bit 63) must be set
 *
 * @returns true if valid, false otherwise
 */
export function validateBracket(hex: string): hex is `0x${string}` {
  // Must be 0x + 16 hex chars
  if (!/^0x[0-9a-fA-F]{16}$/.test(hex)) {
    return false;
  }

  // Sentinel bit (MSB, bit 63) must be set.
  // The first hex digit after 0x must be >= 8 (i.e. top bit of first nibble set).
  const firstNibble = parseInt(hex[2], 16);
  return firstNibble >= 8;
}
