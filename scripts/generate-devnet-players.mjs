import { access, readFile } from "node:fs/promises";
import { resolve } from "node:path";
import {
  createKeyPairFromBytes,
  generateKeyPair,
  getAddressFromPublicKey,
  writeKeyPair,
} from "@solana/kit";

async function loadOrCreatePlayer(name) {
  const keypairPath = resolve(".anchor", `devnet-player-${name}.json`);
  try {
    await access(keypairPath);
    const bytes = new Uint8Array(JSON.parse(await readFile(keypairPath, "utf8")));
    return { address: await getAddressFromPublicKey((await createKeyPairFromBytes(bytes)).publicKey), created: false, keypairPath };
  } catch (error) {
    if (error && typeof error === "object" && "code" in error && error.code !== "ENOENT") throw error;
    const keyPair = await generateKeyPair(true);
    await writeKeyPair(keyPair, keypairPath);
    return { address: await getAddressFromPublicKey(keyPair.publicKey), created: true, keypairPath };
  }
}

for (const name of ["a", "b"]) {
  const player = await loadOrCreatePlayer(name);
  console.log(`${player.created ? "Created" : "Using"} player ${name.toUpperCase()}: ${player.keypairPath}`);
  console.log(`Player ${name.toUpperCase()} address: ${player.address}`);
}
