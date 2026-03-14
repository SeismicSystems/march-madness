/**
 * Integration tests for MarchMadness contract.
 * Requires a running sanvil node at localhost:8545.
 *
 * Uses the @march-madness/client library (MarchMadnessPublicClient,
 * MarchMadnessUserClient, MarchMadnessOwnerClient) for all contract
 * interactions instead of raw wallet/publicClient calls.
 *
 * Run: bun test test/integration.test.ts
 */

import { describe, test, expect, beforeAll } from "bun:test";
import type { Address } from "viem";
import { parseEther } from "viem";
import type { ShieldedPublicClient } from "seismic-viem";
import type {
  MarchMadnessPublicClient,
  MarchMadnessUserClient,
  MarchMadnessOwnerClient,
} from "@march-madness/client";

import {
  getDeployerAccount,
  getPlayerAccounts,
  createWallet,
  createPublicClientInstance,
  deployContractViaSforge,
  randomBracket,
  chalkyBracket,
  increaseTime,
  getAbi,
  isSanvilRunning,
  ENTRY_FEE,
  createMMPublicClient,
  createMMUserClient,
  createMMOwnerClient,
  type AnvilAccount,
  type DeployResult,
  type WalletClient,
} from "../src/utils.js";

// ── Shared state across tests ─────────────────────────────────────────

let deployer: AnvilAccount;
let players: AnvilAccount[];
let deployResult: DeployResult;
let publicClient: ShieldedPublicClient;
let contractAddress: Address;

// Client library instances
let mmPublic: MarchMadnessPublicClient;
let mmOwner: MarchMadnessOwnerClient;
let mmUsers: MarchMadnessUserClient[];

// Raw wallet clients kept only for edge-case tests that need raw access
// (e.g. submitting with wrong fee, reading someone else's bracket before deadline)
let playerWallets: WalletClient[];

// ── Setup ─────────────────────────────────────────────────────────────

beforeAll(async () => {
  const running = await isSanvilRunning();
  if (!running) {
    throw new Error(
      "sanvil is not running at localhost:8545. Start it with: sanvil",
    );
  }

  deployer = getDeployerAccount();
  players = getPlayerAccounts().slice(0, 5); // use 5 players
  publicClient = createPublicClientInstance();

  // Deploy contract with 1 hour deadline offset via sforge
  deployResult = await deployContractViaSforge(3600);
  contractAddress = deployResult.contractAddress;

  // Create client library instances
  mmPublic = createMMPublicClient(contractAddress);
  mmOwner = await createMMOwnerClient(deployer.privateKey, contractAddress);
  mmUsers = await Promise.all(
    players.map((p) => createMMUserClient(p.privateKey, contractAddress)),
  );

  // Raw wallet clients for edge-case tests only
  playerWallets = await Promise.all(
    players.map((p) => createWallet(p.privateKey)),
  );
});

// ── Tests ─────────────────────────────────────────────────────────────

describe("MarchMadness Integration", () => {
  describe("Contract Deployment", () => {
    test("contract is deployed with correct parameters", async () => {
      const entryFee = await mmPublic.getEntryFee();
      expect(entryFee).toBe(ENTRY_FEE);

      const deadline = await mmPublic.getSubmissionDeadline();
      expect(deadline).toBe(deployResult.deadline);

      const owner = await mmPublic.getOwner();
      expect(owner.toLowerCase()).toBe(
        deployResult.ownerAddress.toLowerCase(),
      );
    });

    test("entry count starts at 0", async () => {
      const count = await mmPublic.getEntryCount();
      expect(count).toBe(0);
    });
  });

  describe("Bracket Submission", () => {
    test("multiple players submit brackets concurrently", async () => {
      const submitPromises = mmUsers.slice(0, 3).map(
        async (mmUser, i) => {
          const bracket =
            i === 2 ? chalkyBracket() : randomBracket();
          const hash = await mmUser.submitBracket(bracket);
          const receipt =
            await publicClient.waitForTransactionReceipt({ hash });
          return { index: i, bracket, receipt };
        },
      );

      const results = await Promise.all(submitPromises);
      for (const r of results) {
        expect(r.receipt.status).toBe("success");
      }
    });

    test("entry count reflects submissions", async () => {
      const count = await mmPublic.getEntryCount();
      expect(count).toBe(3);
    });

    test("hasEntry returns true for submitters, false for non-submitters", async () => {
      const has0 = await mmPublic.getHasEntry(players[0].address);
      const has3 = await mmPublic.getHasEntry(players[3].address);
      expect(has0).toBe(true);
      expect(has3).toBe(false);
    });

    test("rejects submission without correct entry fee", async () => {
      // Raw wallet call needed: client library hardcodes ENTRY_FEE,
      // so we must use raw writeContract to test wrong fee.
      const bracket = randomBracket();
      const abi = getAbi();
      await expect(
        playerWallets[3].writeContract({
          address: contractAddress,
          abi,
          functionName: "submitBracket",
          args: [bracket],
          value: parseEther("0.5"),
        }),
      ).rejects.toThrow();
    });

    test("rejects double submission from same address", async () => {
      const bracket = randomBracket();
      await expect(
        mmUsers[0].submitBracket(bracket),
      ).rejects.toThrow();
    });
  });

  describe("Tags", () => {
    test("player sets a tag", async () => {
      const hash = await mmUsers[0].setTag("Duke4Lyfe");
      const receipt = await publicClient.waitForTransactionReceipt({
        hash,
      });
      expect(receipt.status).toBe("success");

      const tag = await mmPublic.getTag(players[0].address);
      expect(tag).toBe("Duke4Lyfe");
    });

    test("player without bracket cannot set tag", async () => {
      await expect(
        mmUsers[3].setTag("NooBracket"),
      ).rejects.toThrow();
    });
  });

  describe("Bracket Updates", () => {
    test("player updates their bracket", async () => {
      const newBracket = randomBracket();
      const hash = await mmUsers[0].updateBracket(newBracket);
      const receipt = await publicClient.waitForTransactionReceipt({
        hash,
      });
      expect(receipt.status).toBe("success");
    });

    test("player without bracket cannot update", async () => {
      const bracket = randomBracket();
      await expect(
        mmUsers[3].updateBracket(bracket),
      ).rejects.toThrow();
    });
  });

  describe("Reading Brackets (Before Deadline)", () => {
    test("player can read own bracket via signed read", async () => {
      const bracket = await mmUsers[0].getMyBracket();

      // Should return a non-zero bytes8 value with sentinel set
      expect(bracket).toBeTruthy();
      const firstByte = parseInt(bracket.slice(2, 4), 16);
      expect(firstByte & 0x80).toBe(0x80);
    });

    test("another player cannot read someone else's bracket before deadline", async () => {
      // Raw wallet call needed: the client library's getMyBracket() only reads
      // the caller's own bracket. To test the contract's access control, we need
      // a raw signed read of another player's bracket.
      const abi = getAbi();
      await expect(
        playerWallets[1].readContract({
          address: contractAddress,
          abi,
          functionName: "getBracket",
          args: [players[0].address],
        }),
      ).rejects.toThrow();
    });
  });

  describe("Post-Deadline Flow", () => {
    test("fast-forward past deadline", async () => {
      // Move time forward by 2 hours (past the 1-hour deadline)
      await increaseTime(7200);

      const block = await publicClient.getBlock();
      expect(block.timestamp).toBeGreaterThan(deployResult.deadline);
    });

    test("cannot submit bracket after deadline", async () => {
      const bracket = randomBracket();
      await expect(
        mmUsers[3].submitBracket(bracket),
      ).rejects.toThrow();
    });

    test("cannot update bracket after deadline", async () => {
      const bracket = randomBracket();
      await expect(
        mmUsers[0].updateBracket(bracket),
      ).rejects.toThrow();
    });

    test("anyone can read brackets after deadline", async () => {
      // After deadline, transparent read works for anyone
      const bracket = await mmPublic.getBracket(players[0].address);

      expect(bracket).toBeTruthy();
      const firstByte = parseInt(bracket.slice(2, 4), 16);
      expect(firstByte & 0x80).toBe(0x80);
    });
  });

  describe("Results & Scoring", () => {
    // Use the chalky bracket as tournament results, so player 3 gets a perfect score
    const resultsHex = chalkyBracket();

    test("non-owner cannot submit results", async () => {
      // Use a non-owner user client to attempt submitting results.
      // Raw wallet call needed: MarchMadnessUserClient doesn't have submitResults().
      const abi = getAbi();
      await expect(
        playerWallets[0].writeContract({
          address: contractAddress,
          abi,
          functionName: "submitResults",
          args: [resultsHex],
        }),
      ).rejects.toThrow();
    });

    test("owner submits results", async () => {
      const hash = await mmOwner.submitResults(resultsHex);
      const receipt = await publicClient.waitForTransactionReceipt({
        hash,
      });
      expect(receipt.status).toBe("success");

      const results = await mmPublic.getResults();
      expect(results).toBe(resultsHex);
    });

    test("cannot submit results twice", async () => {
      await expect(
        mmOwner.submitResults(resultsHex),
      ).rejects.toThrow();
    });

    test("score all submitted brackets", async () => {
      // Score players 0, 1, 2
      for (let i = 0; i < 3; i++) {
        const hash = await mmOwner.scoreBracket(players[i].address);
        const receipt =
          await publicClient.waitForTransactionReceipt({ hash });
        expect(receipt.status).toBe("success");

        const isScored = await mmPublic.getIsScored(players[i].address);
        expect(isScored).toBe(true);
      }
    });

    test("player 3 (chalky bracket) has the highest score of 192", async () => {
      const score = await mmPublic.getScore(players[2].address);
      const winningScore = await mmPublic.getWinningScore();

      // Player 3 submitted chalkyBracket which matches results exactly -> 192
      expect(score).toBe(192);
      expect(winningScore).toBe(192);
    });

    test("cannot score same bracket twice", async () => {
      await expect(
        mmOwner.scoreBracket(players[0].address),
      ).rejects.toThrow();
    });
  });

  describe("Payout", () => {
    test("cannot collect winnings during scoring window", async () => {
      await expect(
        mmUsers[2].collectWinnings(),
      ).rejects.toThrow();
    });

    test("fast-forward past scoring window (7 days)", async () => {
      await increaseTime(7 * 24 * 60 * 60 + 1);
    });

    test("winner (player 3) collects winnings", async () => {
      const balanceBefore = await publicClient.getBalance({
        address: players[2].address,
      });

      const hash = await mmUsers[2].collectWinnings();
      const receipt = await publicClient.waitForTransactionReceipt({
        hash,
      });
      expect(receipt.status).toBe("success");

      const balanceAfter = await publicClient.getBalance({
        address: players[2].address,
      });

      // Winner receives 3 ETH (3 entries * 1 ETH), minus gas
      expect(balanceAfter).toBeGreaterThan(balanceBefore);
    });

    test("winner cannot collect twice", async () => {
      await expect(
        mmUsers[2].collectWinnings(),
      ).rejects.toThrow();
    });

    test("non-winner cannot collect", async () => {
      await expect(
        mmUsers[0].collectWinnings(),
      ).rejects.toThrow();
    });
  });
});
