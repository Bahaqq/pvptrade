"use client";

import { ClientProvider } from "@solana/react";
import { type ReactNode, useMemo } from "react";
import { createPvpTradeClient } from "../lib/solana-client";

export function SolanaProvider({ children }: { children: ReactNode }) {
  const client = useMemo(() => createPvpTradeClient(), []);

  return <ClientProvider client={client}>{children}</ClientProvider>;
}
