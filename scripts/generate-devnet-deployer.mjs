import { access, readFile } from "node:fs/promises";
import { resolve } from "node:path";
import {
  createKeyPairFromBytes,
  generateKeyPair,
  getAddressFromPublicKey,
  writeKeyPair,
} from "@solana/kit";

const keypairPath = resolve(".anchor", "devnet-deployer.json");

async function loadOrCreateKeyPair() {
  try {
    await access(keypairPath);
    const bytes = new Uint8Array(JSON.parse(await readFile(keypairPath, "utf8")));
    return { keyPair: await createKeyPairFromBytes(bytes), created: false };
  } catch (error) {
    if (error && typeof error === "object" && "code" in error && error.code !== "ENOENT") {
      throw error;
    }

    const keyPair = await generateKeyPair(true);
    await writeKeyPair(keyPair, keypairPath);
    return { keyPair, created: true };
  }
}

const { keyPair, created } = await loadOrCreateKeyPair();
const deployerAddress = await getAddressFromPublicKey(keyPair.publicKey);

console.log(`${created ? "Created" : "Using"} local devnet deployer: ${keypairPath}`);
console.log(`Devnet deployer address: ${deployerAddress}`);
