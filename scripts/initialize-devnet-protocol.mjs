import { readFile } from "node:fs/promises";
import {
  AccountRole,
  address,
  appendTransactionMessageInstruction,
  createKeyPairSignerFromBytes,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  createTransactionMessage,
  getAddressEncoder,
  getProgramDerivedAddress,
  getSignatureFromTransaction,
  pipe,
  sendAndConfirmTransactionFactory,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
} from "@solana/kit";

const PROGRAM_ADDRESS = address("2oQvjoTFEP8pyxhNgtSH7aCpoVfQK7wcWoQSLcQqE3wF");
const DEVNET_USDC_MINT = address("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU");
const SYSTEM_PROGRAM = address("11111111111111111111111111111111");
const INITIALIZE_DISCRIMINATOR = new Uint8Array([188, 233, 252, 106, 134, 146, 202, 91]);
const rpcUrl = process.env.SOLANA_RPC_URL ?? "https://api.devnet.solana.com";
const rpcSubscriptionsUrl = process.env.SOLANA_WS_URL ?? rpcUrl.replace(/^http/, "ws");
const deployerPath = process.env.DEVNET_DEPLOYER_PATH ?? ".anchor/devnet-deployer.json";

const signer = await createKeyPairSignerFromBytes(
  new Uint8Array(JSON.parse(await readFile(deployerPath, "utf8"))),
);
const [protocolConfig] = await getProgramDerivedAddress({
  programAddress: PROGRAM_ADDRESS,
  seeds: [new TextEncoder().encode("protocol")],
});
const rpc = createSolanaRpc(rpcUrl);
const existing = await rpc.getAccountInfo(protocolConfig, { commitment: "confirmed", encoding: "base64" }).send();

if (existing.value) {
  console.log(`Protocol config already initialized: ${protocolConfig}`);
} else {
  const data = new Uint8Array(18);
  data.set(INITIALIZE_DISCRIMINATOR);
  const view = new DataView(data.buffer);
  view.setUint16(8, 200, true);
  view.setBigInt64(10, 300n, true);

  const instruction = {
    programAddress: PROGRAM_ADDRESS,
    accounts: [
      { address: protocolConfig, role: AccountRole.WRITABLE },
      { address: DEVNET_USDC_MINT, role: AccountRole.READONLY },
      { address: signer.address, role: AccountRole.WRITABLE_SIGNER, signer },
      { address: SYSTEM_PROGRAM, role: AccountRole.READONLY },
    ],
    data,
  };

  if (process.argv.includes("--dry-run")) {
    console.log(`Protocol config: ${protocolConfig}`);
    console.log(`Authority: ${signer.address}`);
    console.log(`Instruction bytes: ${Array.from(data).join(",")}`);
  } else {
    const { value: latestBlockhash } = await rpc.getLatestBlockhash({ commitment: "confirmed" }).send();
    const transactionMessage = pipe(
      createTransactionMessage({ version: 0 }),
      (message) => setTransactionMessageFeePayerSigner(signer, message),
      (message) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, message),
      (message) => appendTransactionMessageInstruction(instruction, message),
    );
    const transaction = await signTransactionMessageWithSigners(transactionMessage);
    await sendAndConfirmTransactionFactory({
      rpc,
      rpcSubscriptions: createSolanaRpcSubscriptions(rpcSubscriptionsUrl),
    })(transaction, { commitment: "confirmed" });

    console.log(`Protocol config initialized: ${protocolConfig}`);
    console.log(`Initialization signature: ${getSignatureFromTransaction(transaction)}`);
  }
}
