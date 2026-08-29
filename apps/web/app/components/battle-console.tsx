"use client";

import { useConnectedWallet } from "@solana/kit-plugin-wallet/react";
import { useClient } from "@solana/react";
import { useState } from "react";
import type { PvpTradeClient } from "../lib/solana-client";

type Arena = "safe" | "meme";

export function BattleConsole() {
  const client = useClient<PvpTradeClient>();
  const connected = useConnectedWallet(client);
  const [stake, setStake] = useState("25");
  const [arena, setArena] = useState<Arena>("safe");
  const [preparedId, setPreparedId] = useState<string | null>(null);

  const prepareBattle = () => {
    if (!connected) return;
    const id = crypto.randomUUID().replaceAll("-", "").slice(0, 12).toUpperCase();
    setPreparedId(id);
  };

  return (
    <section className="consoleSection shell" id="battle-console">
      <div className="sectionHeading consoleHeading">
        <span>Devnet battle console</span>
        <h2>Connect once. Prepare an isolated USDC arena.</h2>
      </div>

      <div className="consoleGrid">
        <div className="battleForm">
          <div className="formRow">
            <label htmlFor="stake">Stake per trader</label>
            <div className="amountInput">
              <input
                id="stake"
                inputMode="decimal"
                value={stake}
                onChange={(event) => setStake(event.target.value)}
                aria-describedby="stake-help"
              />
              <span>USDC</span>
            </div>
            <small id="stake-help">Both traders deposit exactly the same six-decimal token amount.</small>
          </div>

          <fieldset>
            <legend>Arena</legend>
            <div className="arenaOptions">
              {(["safe", "meme"] as const).map((option) => (
                <button
                  className={arena === option ? "arenaOption selected" : "arenaOption"}
                  type="button"
                  key={option}
                  onClick={() => setArena(option)}
                >
                  <strong>{option === "safe" ? "Safe arena" : "Meme arena"}</strong>
                  <span>{option === "safe" ? "Curated liquid assets" : "Higher-risk token universe"}</span>
                </button>
              ))}
            </div>
          </fieldset>

          <button className="primaryButton consoleAction" type="button" disabled={!connected} onClick={prepareBattle}>
            {connected ? "Prepare devnet battle" : "Connect wallet to continue"}
          </button>
        </div>

        <aside className="readinessPanel">
          <div className="readinessTop">
            <span className="tag neutral">Transaction readiness</span>
            <span className="networkPill">Devnet</span>
          </div>
          <ul>
            <li className={connected ? "ready" : ""}><span>{connected ? "✓" : "1"}</span> Wallet connected</li>
            <li className="ready"><span>✓</span> Six-decimal settlement mint enforced</li>
            <li className="ready"><span>✓</span> Separate PDA vault per trader</li>
            <li><span>4</span> Program deployment pending</li>
          </ul>
          {preparedId ? (
            <div className="preparedBattle">
              <span>Prepared battle</span>
              <strong>#{preparedId}</strong>
              <p>{stake || "0"} USDC · {arena} arena · 24 hours</p>
            </div>
          ) : (
            <p className="readinessNote">
              Preparation is non-custodial and does not sign a transaction. Live deposits unlock after the audited devnet program is deployed.
            </p>
          )}
        </aside>
      </div>
    </section>
  );
}
