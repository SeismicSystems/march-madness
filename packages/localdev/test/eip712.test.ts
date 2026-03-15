/**
 * EIP-712 integration tests for MarchMadness contract.
 *
 * These tests exercise the messageVersion=2 (EIP-712 typed data) signing path
 * that Privy embedded wallets use in production. The standard integration tests
 * use local accounts (messageVersion=0, raw serialized tx) which don't catch
 * issues in the typed data path.
 *
 * Requires a running sanvil node at localhost:8545 (or RPC_URL).
 *
 * Run: bun test test/eip712.test.ts
 */

import { describe, test, expect, beforeAll } from "bun:test";
import type { Address, Hex } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { sendRawTransaction } from "viem/actions";
import { getAction, extract } from "viem/utils";
import {
  createShieldedWalletClient,
  sanvil,
  seismicTestnetGcp2,
  buildTxSeismicMetadata,
  signSeismicTxTypedData,
  getPlaintextCalldata,
} from "seismic-viem";
import type { ShieldedPublicClient, ShieldedWalletClient } from "seismic-viem";
import { MarchMadnessPublicClient, ENTRY_FEE } from "@march-madness/client";
import { MarchMadnessAbi } from "@march-madness/client";

import {
  getDeployerAccount,
  getPlayerAccounts,
  createPublicClientInstance,
  deployContractViaSforge,
  randomBracket,
  isSanvilRunning,
  getTransport,
  type AnvilAccount,
  type DeployResult,
} from "../src/utils.js";

// ── Shared state ──────────────────────────────────────────────────────

let deployer: AnvilAccount;
let player: AnvilAccount;
let deployResult: DeployResult;
let publicClient: ShieldedPublicClient;
let contractAddress: Address;
let mmPublic: MarchMadnessPublicClient;

const RPC_URL = process.env.RPC_URL || "http://localhost:8545";
const isTestnet = RPC_URL !== "http://localhost:8545";

// ── Helper: send a shielded tx using EIP-712 typed data signing ──────
// This mirrors exactly what happens in production with Privy embedded wallets
// (json-rpc accounts, messageVersion=2).

async function sendShieldedTxViaEip712(opts: {
  privateKey: Hex;
  to: Address;
  data: Hex;
  value?: bigint;
}): Promise<Hex> {
  const { privateKey, to, data, value = 0n } = opts;
  const account = privateKeyToAccount(privateKey);
  const chainId = await publicClient.getChainId();
  const chain = chainId === sanvil.id ? sanvil : seismicTestnetGcp2;

  const walletClient = await createShieldedWalletClient({
    account,
    chain,
    transport: getTransport(),
  });

  // 1. Build seismic metadata with typedDataTx=true to force messageVersion=2
  const metadata = await buildTxSeismicMetadata(walletClient as any, {
    account,
    to,
    value,
    signedRead: false,
    typedDataTx: true,
  });

  // Verify messageVersion is 2 (EIP-712)
  expect(metadata.seismicElements.messageVersion).toBe(2);

  // 2. Encrypt the calldata
  const encryptedCalldata = await walletClient.encrypt(data, metadata);

  // 3. Build the transaction request
  // Skip prepareTransactionRequest (gas estimation) because it simulates the tx
  // at the RPC layer where block.timestamp may differ from sforge's block.timestamp
  // used at deploy time. Instead, provide explicit gas params.
  const chainFormat = walletClient.chain?.formatters?.transactionRequest?.format;
  const gasPrice = await walletClient.getGasPrice();

  const preparedTx = {
    ...extract(
      { ...metadata.seismicElements, type: "seismic" },
      { format: chainFormat },
    ),
    chainId: metadata.legacyFields.chainId,
    data: encryptedCalldata,
    from: account.address,
    nonce: metadata.legacyFields.nonce,
    to,
    value,
    gas: 500_000n,
    gasPrice,
    type: "seismic",
  } as any;

  // 4. Sign via EIP-712 typed data (same as what Privy does)
  const { typedData, signature } = await signSeismicTxTypedData(
    walletClient as any,
    preparedTx,
  );

  // 5. Send via eth_sendRawTransaction with typed data payload
  // seismic-reth accepts both raw hex bytes AND typed data objects
  const action = getAction(
    walletClient as any,
    sendRawTransaction,
    "sendRawTransaction",
  );
  const hash = await action({
    // @ts-ignore — SeismicRawTxRequest enum: Bytes | TypedData
    serializedTransaction: { data: typedData, signature },
  });

  return hash as Hex;
}

// ── Setup ─────────────────────────────────────────────────────────────

beforeAll(async () => {
  const running = await isSanvilRunning();
  if (!running) {
    throw new Error(
      "sanvil is not running at localhost:8545. Start it with: sanvil",
    );
  }

  deployer = getDeployerAccount();
  player = getPlayerAccounts()[0];
  publicClient = createPublicClientInstance();

  // Deploy fresh. Use a very large deadline offset to work around the sforge
  // block.timestamp vs sanvil RPC block.timestamp mismatch.
  // sforge may see a much smaller block.timestamp than the runtime EVM.
  deployResult = await deployContractViaSforge(2_000_000_000);
  contractAddress = deployResult.contractAddress;

  mmPublic = new MarchMadnessPublicClient(publicClient, contractAddress);
});

// ── Tests ─────────────────────────────────────────────────────────────

describe("EIP-712 Typed Data Transactions", () => {
  test("submitBracket via EIP-712 typed data (messageVersion=2)", async () => {
    const bracket = randomBracket();

    // Encode calldata the same way shieldedWriteContract does
    const calldata = getPlaintextCalldata({
      abi: MarchMadnessAbi,
      functionName: "submitBracket",
      args: [bracket],
      address: contractAddress,
    } as any);

    console.log("Sending submitBracket via EIP-712 typed data...");
    console.log("  contract:", contractAddress);
    console.log("  player:", player.address);
    console.log("  bracket:", bracket);

    const hash = await sendShieldedTxViaEip712({
      privateKey: player.privateKey,
      to: contractAddress,
      data: calldata,
      value: ENTRY_FEE,
    });

    console.log("  tx hash:", hash);
    expect(hash).toBeTruthy();
    expect(hash).toMatch(/^0x[0-9a-f]{64}$/);

    // Wait for receipt
    const receipt = await publicClient.waitForTransactionReceipt({ hash });
    console.log("  status:", receipt.status);
    expect(receipt.status).toBe("success");

    // Verify the submission
    const hasEntry = await mmPublic.getHasEntry(player.address);
    expect(hasEntry).toBe(true);

    const count = await mmPublic.getEntryCount();
    expect(count).toBeGreaterThanOrEqual(1);
    console.log("  entry count:", count);
  });
});
