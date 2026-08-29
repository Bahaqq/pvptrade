import { createClient } from "@solana/kit";
import { solanaRpc } from "@solana/kit-plugin-rpc";
import { walletSigner } from "@solana/kit-plugin-wallet";

export const SOLANA_RPC_URL = process.env.NEXT_PUBLIC_SOLANA_RPC_URL ?? "https://api.devnet.solana.com";

export function createPvpTradeClient() {
  return createClient()
    .use(walletSigner({ chain: "solana:devnet" }))
    .use(solanaRpc({ rpcUrl: SOLANA_RPC_URL }));
}

export type PvpTradeClient = ReturnType<typeof createPvpTradeClient>;
