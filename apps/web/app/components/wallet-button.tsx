"use client";

import { useClient } from "@solana/react";
import {
  useConnect,
  useConnectedWallet,
  useDisconnect,
  useWallets,
  useWalletStatus,
} from "@solana/kit-plugin-wallet/react";
import { useEffect, useRef, useState } from "react";
import type { PvpTradeClient } from "../lib/solana-client";

function shortenAddress(value: string) {
  return `${value.slice(0, 4)}…${value.slice(-4)}`;
}

export function WalletButton() {
  const client = useClient<PvpTradeClient>();
  const wallets = useWallets(client);
  const connected = useConnectedWallet(client);
  const status = useWalletStatus(client);
  const { dispatch: connect, error } = useConnect(client);
  const { dispatch: disconnect } = useDisconnect(client);
  const [isOpen, setIsOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function closeOnOutsideClick(event: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    }

    document.addEventListener("mousedown", closeOnOutsideClick);
    return () => document.removeEventListener("mousedown", closeOnOutsideClick);
  }, []);

  if (connected) {
    const walletAddress = connected.account.address;

    return (
      <div className="walletControl" ref={menuRef}>
        <button className="walletButton connected" type="button" onClick={() => setIsOpen(!isOpen)}>
          <span className="walletDot" />
          {shortenAddress(walletAddress)}
        </button>
        {isOpen ? (
          <div className="walletMenu">
            <span className="walletMenuLabel">Connected to devnet</span>
            <code>{walletAddress}</code>
            <button
              type="button"
              onClick={() => {
                disconnect();
                setIsOpen(false);
              }}
            >
              Disconnect
            </button>
          </div>
        ) : null}
      </div>
    );
  }

  return (
    <div className="walletControl" ref={menuRef}>
      <button className="walletButton" type="button" onClick={() => setIsOpen(!isOpen)}>
        Connect wallet
      </button>
      {isOpen ? (
        <div className="walletMenu walletPicker">
          <span className="walletMenuLabel">Choose a Solana wallet</span>
          {wallets.length === 0 ? (
            <p>No wallet detected. Install Phantom, Solflare, or another Wallet Standard wallet.</p>
          ) : (
            wallets.map((wallet) => (
              <button
                type="button"
                key={wallet.name}
                disabled={status === "connecting"}
                onClick={() => {
                  connect(wallet);
                  setIsOpen(false);
                }}
              >
                {wallet.name}
              </button>
            ))
          )}
          {error ? <p className="walletError">{error instanceof Error ? error.message : String(error)}</p> : null}
        </div>
      ) : null}
    </div>
  );
}
