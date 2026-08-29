# ADR 0003: Realised USDC settlement

## Status

Accepted — 29 August 2026

## Decision

The MVP determines final equity from realised native USDC after supported battle assets are settled.

## Consequences

- Arbitrary meme-token oracle coverage is not required for final scoring.
- Settlement execution quality is part of protocol fairness.
- Token eligibility requires a credible exit route.
- Batch/pro-rata handling is needed when both players hold the same illiquid mint.
