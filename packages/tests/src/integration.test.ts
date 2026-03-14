/**
 * Integration tests for MarchMadness contract.
 * Requires a running sanvil node at localhost:8545.
 *
 * Run: bun test src/integration.test.ts
 */

import { describe, test, expect, beforeAll } from "bun:test";
import type { Address } from "viem";
import { parseEther } from "viem";

import {
  getDeployerAccount,
  getPlayerAccounts,
  createWallet,
  createPublicClientInstance,
  createMMPublicClient,
  createMMUserClient,
  createMMOwnerClient,
  deployContractViaSforge,
  deployContractDirect,
  randomBracket,
  chalkyBracket,
  increaseTime,
  MarchMadnessAbi,
  isSanvilRunning,
  ENTRY_FEE,
  MarchMadnessPublicClient,
  MarchMadnessUserClient,
  MarchMadnessOwnerClient,
  type AnvilAccount,
  type DeployResult,
  type WalletClient,
} from "./utils.js";

// ── Shared state across tests ─────────────────────────────────────────

let deployer: AnvilAccount;
let players: AnvilAccount[];
let deploy: DeployResult;
let mmPublic: MarchMadnessPublicClient;
let ownerClient: MarchMadnessOwnerClient;
let playerClients: MarchMadnessUserClient[];
let contractAddress: Address;

// Raw wallet clients kept for edge-case tests (wrong fee, raw treadContract, etc.)
let playerWallets: WalletClient[];
let publicClient: ReturnType<typeof createPublicClientInstance>;

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

  // Deploy contract with 1 hour deadline offset
  // Try sforge first, fall back to direct deploy
  try {
    deploy = await deployContractViaSforge(3600);
  } catch {
    deploy = await deployContractDirect(3600);
  }
  contractAddress = deploy.contractAddress;

  // Create typed client instances
  mmPublic = createMMPublicClient(contractAddress);
  ownerClient = await createMMOwnerClient(contractAddress, deployer.privateKey);
  playerClients = await Promise.all(
    players.map((p) => createMMUserClient(contractAddress, p.privateKey)),
  );

  // Raw wallet clients for edge-case tests that need low-level access
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
      expect(deadline).toBe(deploy.deadline);

      const owner = await mmPublic.getOwner();
      expect(owner.toLowerCase()).toBe(deploy.ownerAddress.toLowerCase());
    });

    test("entry count starts at 0", async () => {
      const count = await mmPublic.getEntryCount();
      expect(count).toBe(0);
    });
  });

  describe("Bracket Submission", () => {
    test("multiple players submit brackets concurrently", async () => {
      const submitPromises = playerClients.slice(0, 3).map(
        async (client, i) => {
          const bracket = i === 2 ? chalkyBracket() : randomBracket();
          const hash = await client.submitBracket(bracket);
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

    test("rejects submission without correct entry fee", async () => {
      // Use raw wallet client to send wrong fee (client library always sends correct fee)
      const bracket = randomBracket();
      await expect(
        playerWallets[3].writeContract({
          address: contractAddress,
          abi: MarchMadnessAbi,
          functionName: "submitBracket",
          args: [bracket],
          value: parseEther("0.5"),
        }),
      ).rejects.toThrow();
    });

    test("rejects double submission from same address", async () => {
      const bracket = randomBracket();
      await expect(
        playerClients[0].submitBracket(bracket),
      ).rejects.toThrow();
    });
  });

  describe("Tags", () => {
    test("player sets a tag", async () => {
      const hash = await playerClients[0].setTag("Duke4Lyfe");
      const receipt = await publicClient.waitForTransactionReceipt({ hash });
      expect(receipt.status).toBe("success");

      const tag = await mmPublic.getTag(players[0].address);
      expect(tag).toBe("Duke4Lyfe");
    });

    test("player without bracket cannot set tag", async () => {
      await expect(
        playerClients[3].setTag("NooBracket"),
      ).rejects.toThrow();
    });
  });

  describe("Bracket Updates", () => {
    test("player updates their bracket", async () => {
      const newBracket = randomBracket();
      const hash = await playerClients[0].updateBracket(newBracket);
      const receipt = await publicClient.waitForTransactionReceipt({ hash });
      expect(receipt.status).toBe("success");
    });

    test("player without bracket cannot update", async () => {
      const bracket = randomBracket();
      await expect(
        playerClients[3].updateBracket(bracket),
      ).rejects.toThrow();
    });
  });

  describe("Reading Brackets (Before Deadline)", () => {
    test("player can read own bracket via signed read", async () => {
      // getMyBracket uses signed read before deadline
      const bracket = await playerClients[0].getMyBracket();

      // Should return a non-zero bytes8 value with sentinel set
      expect(bracket).toBeTruthy();
      const firstByte = parseInt(bracket.slice(2, 4), 16);
      expect(firstByte & 0x80).toBe(0x80);
    });

    test("another player cannot read someone else's bracket before deadline", async () => {
      // Use raw wallet client for cross-user signed read (client library's getMyBracket only reads own bracket)
      await expect(
        playerWallets[1].readContract({
          address: contractAddress,
          abi: MarchMadnessAbi,
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
      expect(block.timestamp).toBeGreaterThan(deploy.deadline);
    });

    test("cannot submit bracket after deadline", async () => {
      const bracket = randomBracket();
      await expect(
        playerClients[3].submitBracket(bracket),
      ).rejects.toThrow();
    });

    test("cannot update bracket after deadline", async () => {
      const bracket = randomBracket();
      await expect(
        playerClients[0].updateBracket(bracket),
      ).rejects.toThrow();
    });

    test("anyone can read brackets after deadline via transparent read", async () => {
      // After deadline, public client can read any bracket
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
      // Use raw wallet client to test non-owner submitting results
      await expect(
        playerWallets[0].writeContract({
          address: contractAddress,
          abi: MarchMadnessAbi,
          functionName: "submitResults",
          args: [resultsHex],
        }),
      ).rejects.toThrow();
    });

    test("owner submits results", async () => {
      const hash = await ownerClient.submitResults(resultsHex);
      const receipt = await publicClient.waitForTransactionReceipt({ hash });
      expect(receipt.status).toBe("success");

      const results = await mmPublic.getResults();
      expect(results).toBe(resultsHex);
    });

    test("cannot submit results twice", async () => {
      await expect(
        ownerClient.submitResults(resultsHex),
      ).rejects.toThrow();
    });

    test("score all submitted brackets", async () => {
      // Score players 0, 1, 2 using the owner client's scoreBracket
      for (let i = 0; i < 3; i++) {
        const hash = await ownerClient.scoreBracket(players[i].address);
        const receipt =
          await publicClient.waitForTransactionReceipt({ hash });
        expect(receipt.status).toBe("success");

        const isScored = await mmPublic.getIsScored(players[i].address);
        expect(isScored).toBe(true);
      }

      // numScored is not exposed by the client classes, use raw ABI read
      const numScored = await publicClient.readContract({
        address: contractAddress,
        abi: MarchMadnessAbi,
        functionName: "numScored",
      });
      expect(numScored).toBe(3n);
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
        ownerClient.scoreBracket(players[0].address),
      ).rejects.toThrow();
    });
  });

  describe("Payout", () => {
    test("cannot collect winnings during scoring window", async () => {
      await expect(
        playerClients[2].collectWinnings(),
      ).rejects.toThrow();
    });

    test("fast-forward past scoring window (7 days)", async () => {
      await increaseTime(7 * 24 * 60 * 60 + 1);
    });

    test("winner (player 3) collects winnings", async () => {
      const balanceBefore = await publicClient.getBalance({
        address: players[2].address,
      });

      const hash = await playerClients[2].collectWinnings();
      const receipt = await publicClient.waitForTransactionReceipt({ hash });
      expect(receipt.status).toBe("success");

      const balanceAfter = await publicClient.getBalance({
        address: players[2].address,
      });

      // Winner receives 3 ETH (3 entries * 1 ETH), minus gas
      expect(balanceAfter).toBeGreaterThan(balanceBefore);
    });

    test("winner cannot collect twice", async () => {
      await expect(
        playerClients[2].collectWinnings(),
      ).rejects.toThrow();
    });

    test("non-winner cannot collect", async () => {
      await expect(
        playerClients[0].collectWinnings(),
      ).rejects.toThrow();
    });
  });
});
