"use client";

import { useConnectedWallet } from "@solana/kit-plugin-wallet/react";
import { useClient } from "@solana/react";
import { useEffect, useState } from "react";
import {
  createRandomBattleId,
  getCreateBattleInstruction,
  getJoinBattleInstruction,
  isPvpTradeProgramDeployed,
  parseBattleId,
  parseUsdcAmount,
  type BattleArena,
} from "../lib/pvp-trade-program";
import type { PvpTradeClient } from "../lib/solana-client";

type ConsoleMode = "create" | "join";
type DeploymentStatus = "checking" | "deployed" | "missing" | "unavailable";

function transactionErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function BattleConsole() {
  const client = useClient<PvpTradeClient>();
  const connected = useConnectedWallet(client);
  const [mode, setMode] = useState<ConsoleMode>("create");
  const [stake, setStake] = useState("25");
  const [arena, setArena] = useState<BattleArena>("safe");
  const [battleId, setBattleId] = useState("");
  const [deployment, setDeployment] = useState<DeploymentStatus>("checking");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [signature, setSignature] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    isPvpTradeProgramDeployed()
      .then((deployed) => {
        if (!cancelled) setDeployment(deployed ? "deployed" : "missing");
      })
      .catch(() => {
        if (!cancelled) setDeployment("unavailable");
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const changeMode = (nextMode: ConsoleMode) => {
    setMode(nextMode);
    setSignature(null);
    setError(null);
    setBattleId("");
  };

  const submitBattle = async () => {
    if (!connected || deployment !== "deployed") return;
    setIsSubmitting(true);
    setSignature(null);
    setError(null);

    try {
      const signer = client.identity;
      let activeBattleId = battleId;
      const instruction =
        mode === "create"
          ? await getCreateBattleInstruction({
              arena,
              battleId: (activeBattleId = createRandomBattleId()),
              challenger: signer,
              stakeMicroUsdc: parseUsdcAmount(stake),
            })
          : await getJoinBattleInstruction({
              battleId: (parseBattleId(activeBattleId), activeBattleId),
              opponent: signer,
            });
      setBattleId(activeBattleId);
      const result = await client.sendTransaction(instruction);
      setSignature(result.context.signature);
    } catch (submissionError) {
      setError(transactionErrorMessage(submissionError));
    } finally {
      setIsSubmitting(false);
    }
  };

  const deploymentLabel = {
    checking: "Checking program deployment",
    deployed: "Program deployed and executable",
    missing: "Program deployment pending",
    unavailable: "Devnet RPC unavailable",
  }[deployment];
  const canSubmit = Boolean(connected) && deployment === "deployed" && !isSubmitting;
  const actionLabel = !connected
    ? "Connect wallet to continue"
    : deployment === "checking"
      ? "Checking devnet program…"
      : deployment !== "deployed"
        ? "Program deployment pending"
        : isSubmitting
          ? "Confirming transaction…"
          : mode === "create"
            ? "Create battle & deposit"
            : "Join battle & deposit";

  return (
    <section className="consoleSection shell" id="battle-console">
      <div className="sectionHeading consoleHeading">
        <span>Devnet battle console</span>
        <h2>Connect once. Enter an isolated USDC arena.</h2>
      </div>

      <div className="consoleGrid">
        <div className="battleForm">
          <div className="consoleModes" aria-label="Battle action">
            {(["create", "join"] as const).map((option) => (
              <button
                className={mode === option ? "selected" : ""}
                key={option}
                onClick={() => changeMode(option)}
                type="button"
              >
                {option === "create" ? "Create battle" : "Join battle"}
              </button>
            ))}
          </div>

          {mode === "create" ? (
            <>
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
            </>
          ) : (
            <div className="formRow joinBattleField">
              <label htmlFor="battle-id">Battle ID</label>
              <input
                className="battleIdInput"
                id="battle-id"
                value={battleId}
                onChange={(event) => setBattleId(event.target.value)}
                placeholder="64-character battle ID"
                spellCheck={false}
              />
              <small>Paste the complete ID shared by the challenger. Your stake is read from the battle account.</small>
            </div>
          )}

          <button className="primaryButton consoleAction" type="button" disabled={!canSubmit} onClick={submitBattle}>
            {actionLabel}
          </button>
          {error ? <p className="transactionError">{error}</p> : null}
        </div>

        <aside className="readinessPanel">
          <div className="readinessTop">
            <span className="tag neutral">Transaction readiness</span>
            <span className="networkPill">Devnet</span>
          </div>
          <ul>
            <li className={connected ? "ready" : ""}><span>{connected ? "✓" : "1"}</span> Wallet connected</li>
            <li className="ready"><span>✓</span> Circle devnet USDC enforced</li>
            <li className="ready"><span>✓</span> Separate PDA vault per trader</li>
            <li className={deployment === "deployed" ? "ready" : ""}>
              <span>{deployment === "deployed" ? "✓" : "4"}</span> {deploymentLabel}
            </li>
          </ul>
          {signature ? (
            <div className="preparedBattle">
              <span>Transaction confirmed</span>
              <strong>#{battleId.slice(0, 12).toUpperCase()}</strong>
              <p>
                <a href={`https://explorer.solana.com/tx/${signature}?cluster=devnet`} target="_blank" rel="noreferrer">
                  View transaction on Solana Explorer ↗
                </a>
              </p>
            </div>
          ) : battleId && mode === "create" ? (
            <div className="preparedBattle">
              <span>Battle ID</span>
              <strong>#{battleId.slice(0, 12).toUpperCase()}</strong>
              <p className="battleIdFull">{battleId}</p>
            </div>
          ) : (
            <p className="readinessNote">
              The client now builds real Anchor transactions. Deposits unlock automatically when the executable program and protocol config are live on devnet.
            </p>
          )}
        </aside>
      </div>
    </section>
  );
}
