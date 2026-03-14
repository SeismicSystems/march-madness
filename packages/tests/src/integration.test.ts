/**
 * Integration tests for MarchMadness contract.
 * Requires a running sanvil node at localhost:8545.
 *
 * Run: bun test src/integration.test.ts
 */

import { describe, test, expect, beforeAll } from "bun:test";
import type { Address } from "viem";
import { parseEther } from "viem";
import type { ShieldedPublicClient } from "seismic-viem";

import {
  getAnvilAccounts,
  getDeployerAccount,
  getPlayerAccounts,
  createWallet,
  createPublicClientInstance,
  deployContractViaSforge,
  deployContractDirect,
  randomBracket,
  chalkyBracket,
  increaseTime,
  getAbi,
  isSanvilRunning,
  ENTRY_FEE,
  type AnvilAccount,
  type DeployResult,
  type WalletClient,
} from "./utils.js";

// ── Shared state across tests ─────────────────────────────────────────

let deployer: AnvilAccount;
let players: AnvilAccount[];
let deploy: DeployResult;
let abi: any;
let publicClient: ShieldedPublicClient;
let ownerWallet: WalletClient;
let playerWallets: WalletClient[];
let contractAddress: Address;

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
  abi = getAbi();
  publicClient = createPublicClientInstance();

  // Deploy contract with 1 hour deadline offset
  // Try sforge first, fall back to direct deploy
  try {
    deploy = await deployContractViaSforge(3600);
  } catch {
    deploy = await deployContractDirect(3600);
  }
  contractAddress = deploy.contractAddress;

  // Create wallet clients: owner + 5 players
  ownerWallet = await createWallet(deployer.privateKey);
  playerWallets = await Promise.all(
    players.map((p) => createWallet(p.privateKey)),
  );
});

// ── Tests ─────────────────────────────────────────────────────────────

describe("MarchMadness Integration", () => {
  describe("Contract Deployment", () => {
    test("contract is deployed with correct parameters", async () => {
      const entryFee = await publicClient.readContract({
        address: contractAddress,
        abi,
        functionName: "entryFee",
      });
      expect(entryFee).toBe(ENTRY_FEE);

      const deadline = await publicClient.readContract({
        address: contractAddress,
        abi,
        functionName: "submissionDeadline",
      });
      expect(deadline).toBe(deploy.deadline);

      const owner = await publicClient.readContract({
        address: contractAddress,
        abi,
        functionName: "owner",
      });
      expect((owner as string).toLowerCase()).toBe(
        deploy.ownerAddress.toLowerCase(),
      );
    });

    test("entry count starts at 0", async () => {
      const count = await publicClient.readContract({
        address: contractAddress,
        abi,
        functionName: "getEntryCount",
      });
      expect(count).toBe(0);
    });
  });

  describe("Bracket Submission", () => {
    test("multiple players submit brackets concurrently", async () => {
      const submitPromises = playerWallets.slice(0, 3).map(
        async (wallet, i) => {
          const bracket =
            i === 2 ? chalkyBracket() : randomBracket();
          const hash = await wallet.writeContract({
            address: contractAddress,
            abi,
            functionName: "submitBracket",
            args: [bracket],
            value: ENTRY_FEE,
          });
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
      const count = await publicClient.readContract({
        address: contractAddress,
        abi,
        functionName: "getEntryCount",
      });
      expect(count).toBe(3);
    });

    test("rejects submission without correct entry fee", async () => {
      const bracket = randomBracket();
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
        playerWallets[0].writeContract({
          address: contractAddress,
          abi,
          functionName: "submitBracket",
          args: [bracket],
          value: ENTRY_FEE,
        }),
      ).rejects.toThrow();
    });
  });

  describe("Tags", () => {
    test("player sets a tag", async () => {
      const hash = await playerWallets[0].writeContract({
        address: contractAddress,
        abi,
        functionName: "setTag",
        args: ["Duke4Lyfe"],
      });

      const receipt = await publicClient.waitForTransactionReceipt({
        hash,
      });
      expect(receipt.status).toBe("success");

      const tag = await publicClient.readContract({
        address: contractAddress,
        abi,
        functionName: "getTag",
        args: [players[0].address],
      });
      expect(tag).toBe("Duke4Lyfe");
    });

    test("player without bracket cannot set tag", async () => {
      await expect(
        playerWallets[3].writeContract({
          address: contractAddress,
          abi,
          functionName: "setTag",
          args: ["NooBracket"],
        }),
      ).rejects.toThrow();
    });
  });

  describe("Bracket Updates", () => {
    test("player updates their bracket", async () => {
      const newBracket = randomBracket();

      const hash = await playerWallets[0].writeContract({
        address: contractAddress,
        abi,
        functionName: "updateBracket",
        args: [newBracket],
      });

      const receipt = await publicClient.waitForTransactionReceipt({
        hash,
      });
      expect(receipt.status).toBe("success");
    });

    test("player without bracket cannot update", async () => {
      const bracket = randomBracket();
      await expect(
        playerWallets[3].writeContract({
          address: contractAddress,
          abi,
          functionName: "updateBracket",
          args: [bracket],
        }),
      ).rejects.toThrow();
    });
  });

  describe("Reading Brackets (Before Deadline)", () => {
    test("player can read own bracket via signed read", async () => {
      const bracket = await playerWallets[0].readContract({
        address: contractAddress,
        abi,
        functionName: "getBracket",
        args: [players[0].address],
      });

      // Should return a non-zero bytes8 value with sentinel set
      expect(bracket).toBeTruthy();
      const bracketHex = bracket as `0x${string}`;
      const firstByte = parseInt(bracketHex.slice(2, 4), 16);
      expect(firstByte & 0x80).toBe(0x80);
    });

    test("another player cannot read someone else's bracket before deadline", async () => {
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
      expect(block.timestamp).toBeGreaterThan(deploy.deadline);
    });

    test("cannot submit bracket after deadline", async () => {
      const bracket = randomBracket();
      await expect(
        playerWallets[3].writeContract({
          address: contractAddress,
          abi,
          functionName: "submitBracket",
          args: [bracket],
          value: ENTRY_FEE,
        }),
      ).rejects.toThrow();
    });

    test("cannot update bracket after deadline", async () => {
      const bracket = randomBracket();
      await expect(
        playerWallets[0].writeContract({
          address: contractAddress,
          abi,
          functionName: "updateBracket",
          args: [bracket],
        }),
      ).rejects.toThrow();
    });

    test("anyone can read brackets after deadline via treadContract", async () => {
      // Player 4 (who never submitted) reads player 1's bracket
      const bracket = await playerWallets[3].treadContract({
        address: contractAddress,
        abi,
        functionName: "getBracket",
        args: [players[0].address],
      });

      expect(bracket).toBeTruthy();
      const bracketHex = bracket as `0x${string}`;
      const firstByte = parseInt(bracketHex.slice(2, 4), 16);
      expect(firstByte & 0x80).toBe(0x80);
    });
  });

  describe("Results & Scoring", () => {
    // Use the chalky bracket as tournament results, so player 3 gets a perfect score
    const resultsHex = chalkyBracket();

    test("non-owner cannot submit results", async () => {
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
      const hash = await ownerWallet.writeContract({
        address: contractAddress,
        abi,
        functionName: "submitResults",
        args: [resultsHex],
      });

      const receipt = await publicClient.waitForTransactionReceipt({
        hash,
      });
      expect(receipt.status).toBe("success");

      const results = await publicClient.readContract({
        address: contractAddress,
        abi,
        functionName: "results",
      });
      expect(results).toBe(resultsHex);
    });

    test("cannot submit results twice", async () => {
      await expect(
        ownerWallet.writeContract({
          address: contractAddress,
          abi,
          functionName: "submitResults",
          args: [resultsHex],
        }),
      ).rejects.toThrow();
    });

    test("score all submitted brackets", async () => {
      // Score players 0, 1, 2
      for (let i = 0; i < 3; i++) {
        const hash = await ownerWallet.twriteContract({
          address: contractAddress,
          abi,
          functionName: "scoreBracket",
          args: [players[i].address],
        });

        const receipt =
          await publicClient.waitForTransactionReceipt({ hash });
        expect(receipt.status).toBe("success");

        const isScored = await publicClient.readContract({
          address: contractAddress,
          abi,
          functionName: "getIsScored",
          args: [players[i].address],
        });
        expect(isScored).toBe(true);
      }

      const numScored = await publicClient.readContract({
        address: contractAddress,
        abi,
        functionName: "numScored",
      });
      expect(numScored).toBe(3n);
    });

    test("player 3 (chalky bracket) has the highest score of 192", async () => {
      const score = (await publicClient.readContract({
        address: contractAddress,
        abi,
        functionName: "getScore",
        args: [players[2].address],
      })) as number;

      const winningScore = (await publicClient.readContract({
        address: contractAddress,
        abi,
        functionName: "winningScore",
      })) as number;

      // Player 3 submitted chalkyBracket which matches results exactly -> 192
      expect(score).toBe(192);
      expect(winningScore).toBe(192);
    });

    test("cannot score same bracket twice", async () => {
      await expect(
        ownerWallet.twriteContract({
          address: contractAddress,
          abi,
          functionName: "scoreBracket",
          args: [players[0].address],
        }),
      ).rejects.toThrow();
    });
  });

  describe("Payout", () => {
    test("cannot collect winnings during scoring window", async () => {
      await expect(
        playerWallets[2].writeContract({
          address: contractAddress,
          abi,
          functionName: "collectWinnings",
        }),
      ).rejects.toThrow();
    });

    test("fast-forward past scoring window (7 days)", async () => {
      await increaseTime(7 * 24 * 60 * 60 + 1);
    });

    test("winner (player 3) collects winnings", async () => {
      const balanceBefore = await publicClient.getBalance({
        address: players[2].address,
      });

      const hash = await playerWallets[2].writeContract({
        address: contractAddress,
        abi,
        functionName: "collectWinnings",
      });

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
        playerWallets[2].writeContract({
          address: contractAddress,
          abi,
          functionName: "collectWinnings",
        }),
      ).rejects.toThrow();
    });

    test("non-winner cannot collect", async () => {
      await expect(
        playerWallets[0].writeContract({
          address: contractAddress,
          abi,
          functionName: "collectWinnings",
        }),
      ).rejects.toThrow();
    });
  });
});
