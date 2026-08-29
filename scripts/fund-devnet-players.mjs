import { readFile } from "node:fs/promises";
import {
  AccountRole,
  address,
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
} from "@solana/kit";

const SYSTEM_PROGRAM = address("11111111111111111111111111111111");
const LAMPORTS_PER_PLAYER = 50_000_000n;
const rpcUrl = process.env.SOLANA_RPC_URL ?? "https://api.devnet.solana.com";
const rpcSubscriptionsUrl = process.env.SOLANA_WS_URL ?? rpcUrl.replace(/^http/, "ws");

async function readSigner(path) {
  return createKeyPairSignerFromBytes(new Uint8Array(JSON.parse(await readFile(path, "utf8"))));
}

function getTransferInstruction(sender, recipient) {
  const data = new Uint8Array(12);
  const view = new DataView(data.buffer);
  view.setUint32(0, 2, true);
  view.setBigUint64(4, LAMPORTS_PER_PLAYER, true);
  return {
    programAddress: SYSTEM_PROGRAM,
    accounts: [
      { address: sender.address, role: AccountRole.WRITABLE_SIGNER, signer: sender },
      { address: recipient, role: AccountRole.WRITABLE },
    ],
    data,
  };
}

const [deployer, playerA, playerB] = await Promise.all([
  readSigner(".anchor/devnet-deployer.json"),
  readSigner(".anchor/devnet-player-a.json"),
  readSigner(".anchor/devnet-player-b.json"),
]);
const rpc = createSolanaRpc(rpcUrl);
const balances = await Promise.all(
  [playerA.address, playerB.address].map((player) => rpc.getBalance(player, { commitment: "confirmed" }).send()),
);
const recipients = [playerA.address, playerB.address].filter((_, index) => balances[index].value < LAMPORTS_PER_PLAYER);

if (recipients.length === 0) {
  console.log("Both players already have enough devnet SOL.");
} else {
  const { value: latestBlockhash } = await rpc.getLatestBlockhash({ commitment: "confirmed" }).send();
  const transactionMessage = pipe(
    createTransactionMessage({ version: 0 }),
    (message) => setTransactionMessageFeePayerSigner(deployer, message),
    (message) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, message),
    (message) => appendTransactionMessageInstructions(
      recipients.map((recipient) => getTransferInstruction(deployer, recipient)),
      message,
    ),
  );
  const transaction = await signTransactionMessageWithSigners(transactionMessage);
  await sendAndConfirmTransactionFactory({
    rpc,
    rpcSubscriptions: createSolanaRpcSubscriptions(rpcSubscriptionsUrl),
  })(transaction, { commitment: "confirmed" });
  console.log(`Funded ${recipients.length} player wallet(s).`);
  console.log(`Funding signature: ${getSignatureFromTransaction(transaction)}`);
}

for (const player of [playerA, playerB]) {
  const balance = await rpc.getBalance(player.address, { commitment: "confirmed" }).send();
  console.log(`${player.address}: ${Number(balance.value) / 1_000_000_000} SOL`);
}
