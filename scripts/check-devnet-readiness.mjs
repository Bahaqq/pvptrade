import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import {
  address,
  createKeyPairFromBytes,
  createSolanaRpc,
  getAddressFromPublicKey,
  getProgramDerivedAddress,
} from "@solana/kit";

const expectedProgramAddress = address("2oQvjoTFEP8pyxhNgtSH7aCpoVfQK7wcWoQSLcQqE3wF");
const rpcUrl = process.env.SOLANA_RPC_URL ?? "https://api.devnet.solana.com";

async function readAddress(path) {
  const bytes = new Uint8Array(JSON.parse(await readFile(resolve(path), "utf8")));
  const keyPair = await createKeyPairFromBytes(bytes);
  return getAddressFromPublicKey(keyPair.publicKey);
}

const [deployerAddress, programAddress] = await Promise.all([
  readAddress(".anchor/devnet-deployer.json"),
  readAddress(".anchor/pvp_trade-program-keypair.json"),
]);
if (programAddress !== expectedProgramAddress) {
  throw new Error(`Program key mismatch: expected ${expectedProgramAddress}, received ${programAddress}.`);
}

const [protocolConfig] = await getProgramDerivedAddress({
  programAddress: expectedProgramAddress,
  seeds: [new TextEncoder().encode("protocol")],
});
const rpc = createSolanaRpc(rpcUrl);
const [balance, programAccount, protocolAccount] = await Promise.all([
  rpc.getBalance(deployerAddress, { commitment: "confirmed" }).send(),
  rpc.getAccountInfo(expectedProgramAddress, { commitment: "confirmed", encoding: "base64" }).send(),
  rpc.getAccountInfo(protocolConfig, { commitment: "confirmed", encoding: "base64" }).send(),
]);

console.log(`Deployer: ${deployerAddress}`);
console.log(`Deployer balance: ${Number(balance.value) / 1_000_000_000} SOL`);
console.log(`Program address: ${programAddress}`);
console.log(`Program deployed: ${programAccount.value?.executable === true ? "yes" : "no"}`);
console.log(`Protocol initialized: ${protocolAccount.value ? "yes" : "no"}`);
