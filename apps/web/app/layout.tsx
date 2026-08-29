import type { Metadata } from "next";
import type { ReactNode } from "react";
import { SolanaProvider } from "./components/solana-provider";
import "./globals.css";

export const metadata: Metadata = {
  title: "PVP Trade — Onchain trading battles",
  description: "Equal capital. One arena. Winner takes the remaining pool.",
};

export default function RootLayout({ children }: Readonly<{ children: ReactNode }>) {
  return (
    <html lang="en">
      <body>
        <SolanaProvider>{children}</SolanaProvider>
      </body>
    </html>
  );
}
