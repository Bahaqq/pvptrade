import { readFile } from "node:fs/promises";
import {
  appendTransactionMessageInstructions,
  createKeyPairSignerFromBytes,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  createTransactionMessage,
  getSignatureFromTransaction,
  pipe,
  sendAndConfirmTransactionFactory,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
  type Address,
  type Instruction,
  type InstructionWithSigners,
  type TransactionSigner,
} from "@solana/kit";
import { findAssociatedTokenPda, TOKEN_PROGRAM_ADDRESS } from "@solana-program/token";
import {
  DEVNET_USDC_MINT_ADDRESS,
  deriveBattleAddresses,
  getCreateBattleInstruction,
  getJoinBattleInstruction,
} from "../apps/web/app/lib/pvp-trade-program";

const STAKE_MICRO_USDC = 5_000_000n;
const rpcUrl = process.env.SOLANA_RPC_URL ?? "https://api.devnet.solana.com";
const rpcSubscriptionsUrl = process.env.SOLANA_WS_URL ?? rpcUrl.replace(/^http/, "ws");
const rpc = createSolanaRpc(rpcUrl);

async function readSigner(path: string): Promise<TransactionSigner> {
  const bytes = new Uint8Array(JSON.parse(await readFile(path, "utf8")) as number[]);
  return createKeyPairSignerFromBytes(bytes);
}

async function getUsdcAccount(owner: Address) {
  const [tokenAccount] = await findAssociatedTokenPda({
    mint: DEVNET_USDC_MINT_ADDRESS,
    owner,
    tokenProgram: TOKEN_PROGRAM_ADDRESS,
  });
  return tokenAccount;
}

async function getUsdcBalance(owner: Address): Promise<{ account: Address; amount: bigint }> {
  const account = await getUsdcAccount(owner);
  try {
    const balance = await rpc.getTokenAccountBalance(account, { commitment: "confirmed" }).send();
    return { account, amount: BigInt(balance.value.amount) };
  } catch {
    return { account, amount: 0n };
  }
}

async function sendInstruction(
  feePayer: TransactionSigner,
  instruction: Instruction & InstructionWithSigners,
) {
  const { value: latestBlockhash } = await rpc.getLatestBlockhash({ commitment: "confirmed" }).send();
  const message = pipe(
    createTransactionMessage({ version: 0 }),
    (transactionMessage) => setTransactionMessageFeePayerSigner(feePayer, transactionMessage),
    (transactionMessage) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, transactionMessage),
    (transactionMessage) => appendTransactionMessageInstructions([instruction], transactionMessage),
  );
  const transaction = await signTransactionMessageWithSigners(message);
  await sendAndConfirmTransactionFactory({
    rpc,
    rpcSubscriptions: createSolanaRpcSubscriptions(rpcSubscriptionsUrl),
  })(transaction, { commitment: "confirmed" });
  return getSignatureFromTransaction(transaction);
}

async function main() {
  const [playerA, playerB] = await Promise.all([
    readSigner(".anchor/devnet-player-a.json"),
    readSigner(".anchor/devnet-player-b.json"),
  ]);
  const [balanceA, balanceB] = await Promise.all([
    getUsdcBalance(playerA.address),
    getUsdcBalance(playerB.address),
  ]);

  console.log(`Player A: ${playerA.address} (${Number(balanceA.amount) / 1_000_000} USDC)`);
  console.log(`Player B: ${playerB.address} (${Number(balanceB.amount) / 1_000_000} USDC)`);

  if (balanceA.amount < STAKE_MICRO_USDC || balanceB.amount < STAKE_MICRO_USDC) {
    console.log("\nSmoke test is ready but both players need at least 5 Circle devnet USDC.");
    console.log(`Player A USDC account: ${balanceA.account}`);
    console.log(`Player B USDC account: ${balanceB.account}`);
    process.exitCode = 2;
    return;
  }

  const battleId = Array.from(crypto.getRandomValues(new Uint8Array(32)), (byte) =>
    byte.toString(16).padStart(2, "0"),
  ).join("");
  const { battle } = await deriveBattleAddresses(battleId);

  const createSignature = await sendInstruction(
    playerA,
    await getCreateBattleInstruction({
      arena: "meme",
      battleId,
      challenger: playerA,
      durationSeconds: 86_400n,
      settlementFeeBps: 200,
      stakeMicroUsdc: STAKE_MICRO_USDC,
      tradingLockSeconds: 300n,
    }),
  );
  console.log(`Create signature: ${createSignature}`);

  const joinSignature = await sendInstruction(
    playerB,
    await getJoinBattleInstruction({ battleId, opponent: playerB }),
  );
  console.log(`Join signature: ${joinSignature}`);
  console.log(`Battle ID: ${battleId}`);
  console.log(`Battle account: ${battle}`);
  console.log("Live Create -> Join smoke test passed.");
}

main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
