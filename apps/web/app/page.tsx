import { ARENA_TYPE, createBattle, formatMicroUsdc } from "@pvptrade/protocol";

const previewBattle = createBattle({
  id: "preview-001",
  challenger: "7Xk...pvp",
  stakeMicroUsdc: 100_000_000n,
  durationSeconds: 86_400,
  tradingLockSeconds: 300,
  settlementFeeBps: 200,
  arena: ARENA_TYPE.MEME,
  createdAt: 0,
});

const stages = [
  { label: "Fund", description: "Equal USDC enters isolated battle vaults." },
  { label: "Trade", description: "Swap eligible Solana tokens through Jupiter." },
  { label: "Settle", description: "Positions resolve to realised USDC equity." },
  { label: "Claim", description: "Higher equity wins the remaining pool." },
];

export default function HomePage() {
  return (
    <main>
      <nav className="nav shell">
        <a className="brand" href="#top" aria-label="PVP Trade home">
          <span className="brandMark">P</span>
          <span>PVP TRADE</span>
        </a>
        <div className="navStatus">
          <span className="pulse" />
          Protocol build 0.1
        </div>
      </nav>

      <section className="hero shell" id="top">
        <div className="eyebrow">Solana-native competitive trading</div>
        <div className="heroGrid">
          <div>
            <h1>
              Same capital.
              <br />
              <span>Better trader wins.</span>
            </h1>
            <p className="heroCopy">
              Enter a time-boxed onchain battle, trade from an isolated vault, and prove your edge
              against one opponent under identical rules.
            </p>
            <div className="heroActions">
              <button type="button" className="primaryButton" disabled>
                Battles coming soon
              </button>
              <a className="textLink" href="#protocol">
                Explore the protocol <span aria-hidden="true">↘</span>
              </a>
            </div>
          </div>

          <aside className="battleCard" aria-label="Battle preview">
            <div className="cardHeader">
              <span className="tag meme">Meme arena</span>
              <span className="muted">24:00:00</span>
            </div>
            <div className="versusRow">
              <div className="player">
                <span className="avatar avatarA">A</span>
                <div>
                  <strong>7Xk...pvp</strong>
                  <small>Challenger</small>
                </div>
              </div>
              <span className="versus">VS</span>
              <div className="player playerRight">
                <div>
                  <strong>Waiting</strong>
                  <small>Open seat</small>
                </div>
                <span className="avatar avatarB">?</span>
              </div>
            </div>
            <div className="stakeBlock">
              <span>Stake per trader</span>
              <strong>{formatMicroUsdc(previewBattle.terms.stakeMicroUsdc)}</strong>
            </div>
            <div className="poolLine">
              <span>Starting pool</span>
              <span>200 USDC</span>
            </div>
            <div className="poolLine">
              <span>Settlement</span>
              <span>Winner takes remaining pool</span>
            </div>
            <div className="cardFooter">
              <span className="shield">◇</span>
              Program-controlled vaults
            </div>
          </aside>
        </div>
      </section>

      <section className="ticker" aria-label="Protocol principles">
        <div>
          <span>Equal stake</span>
          <i>◆</i>
          <span>Real DEX execution</span>
          <i>◆</i>
          <span>Realised P&amp;L</span>
          <i>◆</i>
          <span>Transparent settlement</span>
          <i>◆</i>
          <span>Solana speed</span>
        </div>
      </section>

      <section className="protocol shell" id="protocol">
        <div className="sectionHeading">
          <span>How a battle resolves</span>
          <h2>Trading skill, reduced to four verifiable steps.</h2>
        </div>
        <div className="stageGrid">
          {stages.map((stage, index) => (
            <article className="stage" key={stage.label}>
              <div className="stageNumber">0{index + 1}</div>
              <h3>{stage.label}</h3>
              <p>{stage.description}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="buildStrip">
        <div className="shell buildGrid">
          <div>
            <span className="tag neutral">Current milestone</span>
            <h2>Lifecycle proof of architecture</h2>
          </div>
          <ul>
            <li><span>✓</span> Protocol state machine</li>
            <li><span>✓</span> Executable reference model</li>
            <li><span>→</span> Anchor account constraints</li>
            <li><span>→</span> Jupiter CPI vault swaps</li>
          </ul>
        </div>
      </section>

      <footer className="footer shell">
        <span>PVP Trade / Build 0.1</span>
        <span>Not available for real funds</span>
      </footer>
    </main>
  );
}
