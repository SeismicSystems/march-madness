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
  // Work backwards from games to find achievements
  const championshipGame = games[games.length - 1];
  const runnerUp =
    championshipGame.winner === champion
      ? games[games.length - 1].winner === games[games.length - 2].winner
        ? games[games.length - 3]?.winner ?? ""
        : games[games.length - 2]?.winner ?? ""
      : "";

  // Extract by round from games
  const finalFourGames = games.filter((g) => g.round === 4);
  const eliteEightGames = games.filter((g) => g.round === 3);
  const sweetSixteenGames = games.filter((g) => g.round === 2);
  const roundOf32Games = games.filter((g) => g.round === 1);

  return {
    champion,
    runnerUp: runnerUp || games[games.length - 2]?.winner || "",
    finalFour: finalFourGames.map((g) => g.winner),
    eliteEight: eliteEightGames.map((g) => g.winner),
    sweetSixteen: sweetSixteenGames.map((g) => g.winner),
    roundOf32: roundOf32Games.map((g) => g.winner),
    games,
  };
}
