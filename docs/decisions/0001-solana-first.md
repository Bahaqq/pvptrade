# ADR 0001: Solana-first deployment

## Status

Accepted — 29 August 2026

## Decision

The first PVP Trade implementation will deploy on Solana and use Jupiter as its swap-liquidity layer.

## Consequences

- The program is written in Rust with Anchor.
- Battle assets use SPL token accounts and PDA authorities.
- Native Solana USDC is the accounting and settlement asset.
- Robinhood Chain remains a future, separate Arena rather than an MVP dependency.
