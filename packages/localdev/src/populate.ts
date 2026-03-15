/**
 * Bracket population script for local development.
 *
 * Spawns a sanvil node, deploys the MarchMadness contract via sforge, populates
 * it with brackets/results/scores depending on the phase, then starts the Vite
 * dev server with the contract address injected automatically.
 *
 * Usage:
 *   bun run src/populate.ts                              # default: pre-submission, starts vite
 *   bun run src/populate.ts --phase pre-submission       # deploy with future deadline, no brackets
 *   bun run src/populate.ts --phase post-submission      # deploy, submit brackets, fast-forward, post results
 *   bun run src/populate.ts --phase post-grading         # everything above + score all + fast-forward past scoring window
 *   bun run src/populate.ts --no-vite                    # skip starting vite (e.g. for CI or tests)
 *   bun run src/populate.ts --rpc-url http://localhost:8545
 *   CONTRACT_ADDRESS=0x... bun run src/populate.ts --phase post-submission  # use existing contract
 */

import type { Address } from "viem";
import type { MarchMadnessUserClient, MarchMadnessOwnerClient } from "@march-madness/client";

import {
  isSanvilRunning,
  spawnSanvil,
  getPlayerAccounts,
  getDeployerAccount,
  createPublicClientInstance,
  deployContractViaSforge,
  randomBracket,
  chalkyBracket,
  increaseTime,
  TAG_NAMES,
  createMMPublicClient,
  createMMUserClient,
  createMMOwnerClient,
  type DeployResult,
} from "./utils.js";

// ── CLI Arg Parsing ───────────────────────────────────────────────────

type Phase = "pre-submission" | "post-submission" | "post-grading";

function parseArgs(): { phase: Phase; rpcUrl?: string; noVite: boolean } {
  const args = process.argv.slice(2);
  let phase: Phase = "pre-submission";
  let rpcUrl: string | undefined;
  let noVite = false;

  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--phase" && args[i + 1]) {
      const val = args[i + 1];
      if (
        val === "pre-submission" ||
        val === "post-submission" ||
        val === "post-grading"
      ) {
        phase = val;
      } else {
        console.error(
          `Invalid phase: ${val}. Must be one of: pre-submission, post-submission, post-grading`,
        );
        process.exit(1);
      }
      i++;
    } else if (args[i] === "--rpc-url" && args[i + 1]) {
      rpcUrl = args[i + 1];
      i++;
    } else if (args[i] === "--no-vite") {
      noVite = true;
    } else if (args[i] === "--help" || args[i] === "-h") {
      console.log(`
Usage: bun run src/populate.ts [OPTIONS]

Spawns a sanvil node (if not already running), deploys via sforge, and populates state.
Then starts the Vite dev server with the contract address injected.

Options:
  --phase <phase>       Tournament phase to set up (default: pre-submission)
                          pre-submission   - Deploy with future deadline, no brackets
                          post-submission  - Deploy, submit brackets, fast-forward, post results
                          post-grading     - Everything above + score all + past scoring window
  --rpc-url <url>       RPC URL (default: http://localhost:8545)
  --no-vite             Skip starting the Vite dev server (for CI or tests)
  --help, -h            Show this help

Environment Variables:
  CONTRACT_ADDRESS      Use an existing contract instead of deploying
  RPC_URL               RPC URL (overridden by --rpc-url flag)
  DEADLINE_OFFSET       Deadline offset in seconds (default: 3600 for pre-submission)
`);
      process.exit(0);
    }
  }

  return { phase, rpcUrl, noVite };
}

// ── Config ────────────────────────────────────────────────────────────

const { phase, rpcUrl, noVite } = parseArgs();
if (rpcUrl) {
  process.env.RPC_URL = rpcUrl;
}

const EXISTING_CONTRACT = process.env.CONTRACT_ADDRESS as Address | undefined;

// ── Helpers ───────────────────────────────────────────────────────────

function printStatus(label: string, value: string | number | bigint | boolean) {
  console.log(`  ${label.padEnd(24)} ${value}`);
}

async function printContractState(contractAddress: Address) {
  const mmPublic = createMMPublicClient(contractAddress);
  const publicClient = createPublicClientInstance();

  const entryCount = await mmPublic.getEntryCount();
  const deadline = await mmPublic.getSubmissionDeadline();
  const results = await mmPublic.getResults();
  const block = await publicClient.getBlock();
  const deadlinePassed = block.timestamp > deadline;

  console.log("\n--- Contract State ---");
  printStatus("Contract", contractAddress);
  printStatus("Entry Count", String(entryCount));
  printStatus("Deadline (unix)", String(deadline));
  printStatus("Deadline Passed?", String(deadlinePassed));
  printStatus("Current Block Time", String(block.timestamp));
  printStatus("Results Posted?", results !== "0x0000000000000000" ? "YES" : "NO");
  if (results !== "0x0000000000000000") {
    printStatus("Results", String(results));
  }
  console.log("");
}

// ── Deploy ────────────────────────────────────────────────────────────

async function deploy(deadlineOffset: number): Promise<DeployResult> {
  if (EXISTING_CONTRACT) {
    const deployer = getDeployerAccount();
    const mmPublic = createMMPublicClient(EXISTING_CONTRACT);
    const deadline = await mmPublic.getSubmissionDeadline();
    console.log(`Using existing contract: ${EXISTING_CONTRACT}`);
    return {
      contractAddress: EXISTING_CONTRACT,
      groupsContractAddress: "0x0000000000000000000000000000000000000000" as Address,
      mirrorContractAddress: "0x0000000000000000000000000000000000000000" as Address,
      ownerAddress: deployer.address,
      deadline,
    };
  }

  console.log(`Deploying all contracts via sforge (deadline offset: ${deadlineOffset}s)...`);
  const result = await deployContractViaSforge(deadlineOffset);

  console.log(`MarchMadness deployed at: ${result.contractAddress}`);
  console.log(`BracketGroups deployed at: ${result.groupsContractAddress}`);
  console.log(`BracketMirror deployed at: ${result.mirrorContractAddress}`);
  console.log(`Owner: ${result.ownerAddress}`);
  console.log(`Deadline: ${result.deadline}`);
  return result;
}

// ── Phase Implementations ─────────────────────────────────────────────

async function phasePreSubmission(): Promise<DeployResult> {
  console.log("=== Phase: pre-submission ===");
  console.log("Deploying contract with future deadline (1 hour).");
  console.log("No brackets will be submitted -- use the UI to test submission flow.\n");

  const deadlineOffset = parseInt(process.env.DEADLINE_OFFSET || "3600", 10);
  const result = await deploy(deadlineOffset);

  await printContractState(result.contractAddress);
  console.log("Ready for manual bracket submission via the frontend.");
  return result;
}

async function phasePostSubmission(): Promise<DeployResult> {
  console.log("=== Phase: post-submission ===");
  console.log("Deploying contract, submitting brackets, fast-forwarding past deadline, posting results.\n");

  // Deploy with a short deadline (60s) so we can fast-forward past it
  const deployResult = await deploy(60);
  const { contractAddress } = deployResult;
  const publicClient = createPublicClientInstance();
  const players = getPlayerAccounts();
  const deployer = getDeployerAccount();

  // Create client library instances for each player + owner
  console.log(`\nCreating client instances for ${players.length} players...`);
  const mmUsers: MarchMadnessUserClient[] = await Promise.all(
    players.map((p) => createMMUserClient(p.privateKey, contractAddress)),
  );
  const mmOwner: MarchMadnessOwnerClient = await createMMOwnerClient(deployer.privateKey, contractAddress);
  const mmPublic = createMMPublicClient(contractAddress);

  // Submit brackets concurrently -- player index 2 (player-3) gets the chalky bracket
  console.log("Submitting brackets concurrently...");
  const brackets: Map<Address, `0x${string}`> = new Map();

  const submitResults = await Promise.all(
    mmUsers.map(async (mmUser, i) => {
      const player = players[i];
      const bracket = i === 2 ? chalkyBracket() : randomBracket();
      brackets.set(player.address, bracket);

      try {
        const hash = await mmUser.submitBracket(bracket);
        await publicClient.waitForTransactionReceipt({ hash });
        return { label: player.label, address: player.address, bracket, ok: true, error: "" };
      } catch (err: any) {
        return { label: player.label, address: player.address, bracket, ok: false, error: err.message };
      }
    }),
  );

  for (const r of submitResults) {
    const status = r.ok ? "OK" : `FAIL: ${r.error}`;
    console.log(`  ${r.label.padEnd(12)} ${r.address.slice(0, 10)}... ${r.bracket} [${status}]`);
  }

  // Set tags on first 6 players
  console.log("\nSetting tags...");
  await Promise.all(
    mmUsers.slice(0, Math.min(6, mmUsers.length)).map(
      async (mmUser, i) => {
        const tag = TAG_NAMES[i] || `Player${i + 1}`;
        try {
          const hash = await mmUser.setTag(tag);
          await publicClient.waitForTransactionReceipt({ hash });
          console.log(`  ${players[i].label.padEnd(12)} => "${tag}" [OK]`);
        } catch {
          console.log(`  ${players[i].label.padEnd(12)} => "${tag}" [FAIL]`);
        }
      },
    ),
  );

  // Update brackets for players 0 and 3
  console.log("\nUpdating some brackets...");
  for (const idx of [0, 3]) {
    if (idx >= mmUsers.length) continue;
    const newBracket = randomBracket();
    brackets.set(players[idx].address, newBracket);
    try {
      const hash = await mmUsers[idx].updateBracket(newBracket);
      await publicClient.waitForTransactionReceipt({ hash });
      console.log(`  ${players[idx].label.padEnd(12)} => ${newBracket} [OK]`);
    } catch {
      console.log(`  ${players[idx].label.padEnd(12)} => update [FAIL]`);
    }
  }

  // Fast-forward past deadline
  console.log("\nFast-forwarding past deadline...");
  await increaseTime(120);

  // Submit tournament results (chalky bracket = all higher seeds win)
  console.log("Submitting tournament results (chalky bracket)...");
  const resultsHex = chalkyBracket();
  const hash = await mmOwner.submitResults(resultsHex);
  await publicClient.waitForTransactionReceipt({ hash });
  console.log(`  Results: ${resultsHex} [OK]`);

  // Score a FEW brackets (not all) so devs can test scoring manually
  console.log("\nScoring first 3 brackets (leaving rest for manual testing)...");
  for (let i = 0; i < Math.min(3, players.length); i++) {
    try {
      const hash = await mmOwner.scoreBracket(players[i].address);
      await publicClient.waitForTransactionReceipt({ hash });
      const score = await mmPublic.getScore(players[i].address);
      console.log(`  ${players[i].label.padEnd(12)} score: ${score}`);
    } catch (err: any) {
      console.log(`  ${players[i].label.padEnd(12)} scoring failed: ${err.message}`);
    }
  }

  await printContractState(contractAddress);
  console.log("Remaining brackets are unscored -- test scoring via the UI or CLI.");
  return deployResult;
}

async function phasePostGrading(): Promise<DeployResult> {
  console.log("=== Phase: post-grading ===");
  console.log("Deploying contract, submitting brackets, scoring all, fast-forwarding past scoring window.\n");

  // Deploy with short deadline
  const deployResult = await deploy(60);
  const { contractAddress } = deployResult;
  const publicClient = createPublicClientInstance();
  const players = getPlayerAccounts();
  const deployer = getDeployerAccount();

  // Create client library instances
  console.log(`\nCreating client instances for ${players.length} players...`);
  const mmUsers: MarchMadnessUserClient[] = await Promise.all(
    players.map((p) => createMMUserClient(p.privateKey, contractAddress)),
  );
  const mmOwner: MarchMadnessOwnerClient = await createMMOwnerClient(deployer.privateKey, contractAddress);
  const mmPublic = createMMPublicClient(contractAddress);

  // Submit brackets concurrently
  console.log("Submitting brackets concurrently...");
  await Promise.all(
    mmUsers.map(async (mmUser, i) => {
      const bracket = i === 2 ? chalkyBracket() : randomBracket();
      try {
        const hash = await mmUser.submitBracket(bracket);
        await publicClient.waitForTransactionReceipt({ hash });
        console.log(`  ${players[i].label.padEnd(12)} ${bracket} [OK]`);
      } catch (err: any) {
        console.log(`  ${players[i].label.padEnd(12)} [FAIL: ${err.message}]`);
      }
    }),
  );

  // Set tags
  console.log("\nSetting tags...");
  await Promise.all(
    mmUsers.slice(0, Math.min(6, mmUsers.length)).map(
      async (mmUser, i) => {
        const tag = TAG_NAMES[i] || `Player${i + 1}`;
        try {
          const hash = await mmUser.setTag(tag);
          await publicClient.waitForTransactionReceipt({ hash });
        } catch { /* ignore */ }
      },
    ),
  );

  // Fast-forward past deadline
  console.log("\nFast-forwarding past deadline...");
  await increaseTime(120);

  // Submit results
  console.log("Submitting tournament results...");
  const resultsHex = chalkyBracket();
  let hash = await mmOwner.submitResults(resultsHex);
  await publicClient.waitForTransactionReceipt({ hash });
  console.log(`  Results: ${resultsHex}`);

  // Score ALL brackets
  console.log("\nScoring all brackets...");
  for (let i = 0; i < players.length; i++) {
    try {
      hash = await mmOwner.scoreBracket(players[i].address);
      await publicClient.waitForTransactionReceipt({ hash });
      const score = await mmPublic.getScore(players[i].address);
      console.log(`  ${players[i].label.padEnd(12)} score: ${score}`);
    } catch (err: any) {
      console.log(`  ${players[i].label.padEnd(12)} [FAIL: ${err.message}]`);
    }
  }

  // Fast-forward past scoring window (7 days + 1 second)
  console.log("\nFast-forwarding past scoring window (7 days)...");
  await increaseTime(7 * 24 * 60 * 60 + 1);

  // Print winner info
  const winningScore = await mmPublic.getWinningScore();
  const numWinners = await mmPublic.getNumWinners();
  console.log(`\nWinning score: ${winningScore}`);
  console.log(`Number of winners: ${numWinners}`);

  await printContractState(contractAddress);
  console.log("Winners can now call collectWinnings() via the UI or CLI.");
  return deployResult;
}

// ── Main ──────────────────────────────────────────────────────────────

async function main() {
  console.log("=== March Madness Local Population Script ===\n");

  // Spawn sanvil if not already running
  const alreadyRunning = await isSanvilRunning();
  if (alreadyRunning) {
    console.log("sanvil is already running.\n");
  } else {
    console.log("Spawning sanvil...");
    await spawnSanvil();
    console.log("");
  }

  let result: DeployResult;
  switch (phase) {
    case "pre-submission":
      result = await phasePreSubmission();
      break;
    case "post-submission":
      result = await phasePostSubmission();
      break;
    case "post-grading":
      result = await phasePostGrading();
      break;
  }

  if (noVite) {
    console.log("Done. sanvil is still running. Vite skipped (--no-vite).");
    return;
  }

  // Start Vite dev server with all contract addresses injected
  console.log(`\nStarting Vite dev server (contract: ${result.contractAddress})...\n`);
  const { spawn } = await import("child_process");
  const { resolve } = await import("path");

  const webDir = resolve(import.meta.dir, "../../web");
  const vite = spawn("bun", ["run", "dev"], {
    cwd: webDir,
    stdio: "inherit",
    env: {
      ...process.env,
      VITE_CONTRACT_ADDRESS: result.contractAddress,
      VITE_GROUPS_CONTRACT_ADDRESS: result.groupsContractAddress,
      VITE_MIRROR_CONTRACT_ADDRESS: result.mirrorContractAddress,
      VITE_CHAIN_ID: String(31337), // sanvil
    },
  });

  // Forward SIGINT to vite so ctrl-c shuts down cleanly
  process.on("SIGINT", () => {
    vite.kill("SIGINT");
    process.exit(0);
  });

  // Keep the process alive while vite runs
  await new Promise<void>((resolve) => {
    vite.on("close", () => resolve());
  });
}

main().catch((err) => {
  console.error("Fatal error:", err);
  process.exit(1);
});
