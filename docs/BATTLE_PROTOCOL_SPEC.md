# Battle Protocol Specification v0.1

> Status: Draft
> Date: 29 August 2026
> Scope: 1v1 spot-trading proof of architecture

This document defines the rules that every PVP Trade implementation must follow. `MUST`, `MUST NOT`, `SHOULD`, and `MAY` are normative requirements.

## 1. Objective

A battle compares two traders who start with equal USDC capital and trade under the same constraints. At settlement, the player with the greater realised USDC equity wins the remaining combined pool after the disclosed protocol fee.

## 2. Roles

| Role | Responsibility |
|---|---|
| Challenger | Creates the battle and deposits the first equal stake |
| Opponent | Accepts the terms and deposits the matching stake |
| Battle program | Enforces state, time, authority and custody rules |
| Quote coordinator | Fetches routes but cannot move funds or select a winner |
| Settlement keeper | Triggers permissionless settlement work |
| Protocol multisig | Changes bounded configuration and can pause unsafe actions |

No off-chain role may unilaterally choose the winner or redirect a player's assets.

## 3. Battle terms

Terms are immutable after an opponent joins.

Required terms:

- Unique 32-byte battle identifier
- Challenger public key
- Opponent public key once joined
- Equal stake in native Solana USDC base units
- Trading duration in seconds
- Trading-lock duration in seconds
- Arena type
- Settlement fee in basis points, snapshotted at creation
- Creation, start, trading-end and settlement timestamps

Initial proof-of-architecture constraints:

- Stake MUST be greater than zero.
- Duration MUST be positive.
- Challenger and opponent MUST be different public keys.
- Fee MUST NOT exceed the protocol-configured maximum.
- Only one settlement asset is supported: native Solana USDC.

## 4. State machine

```text
OPEN -> FUNDED -> ACTIVE -> TRADING_LOCKED -> SETTLING -> RESOLVED -> CLAIMED
  |        |
  +------> CANCELLED
           +------> REFUNDED
```

Allowed transitions:

| Current state | Next state | Required condition |
|---|---|---|
| Open | Funded | Distinct opponent joins and equal stake is secured |
| Open | Cancelled | Challenger cancels before an opponent joins |
| Funded | Active | Both stakes exist and the start action succeeds |
| Funded | Refunded | Start deadline expires or a defined safety condition occurs |
| Active | TradingLocked | Trading cutoff timestamp is reached |
| TradingLocked | Settling | No new risk-increasing trades can execute |
| Settling | Resolved | All supported assets have deterministic USDC outcomes |
| Resolved | Claimed | Payout reaches the recorded winner or draw recipients |

Any transition not listed above MUST fail.

## 5. Time rules

- Program time MUST come from Solana's `Clock` sysvar.
- Client time MUST NOT be authoritative.
- `trading_end_at = starts_at + duration_seconds`.
- `trading_lock_at = trading_end_at - trading_lock_seconds`.
- A transaction is accepted based on the timestamp observed during program execution, not wallet-signing time.
- Trading lock MUST occur before or at trading end.

The exact MVP lock window remains an open product parameter.

## 6. Custody invariants

The production vault implementation MUST satisfy all of the following:

1. Each player has a separate PDA authority scoped to one battle.
2. Battle assets cannot be withdrawn while the battle is active.
3. Swap output can only arrive at a token account owned by the same player's battle PDA.
4. A route cannot designate an arbitrary destination, fee account, close authority, or refund recipient.
5. A player cannot access the opponent's vault.
6. Protocol fees cannot be collected before the fee-triggering event defined in the terms.
7. Admin pause authority cannot seize battle funds.

Version 0.2 implements the initial settlement-asset subset of these rules: a protocol-pinned six-decimal mint, one PDA token vault per trader, equal `transfer_checked` deposits, and challenger refund on open-battle cancellation. Trade-asset custody and active-battle withdrawal rejection remain part of the next proof slice.

## 7. Trading rules

- MVP trading is spot-only.
- Swaps MUST use an approved Jupiter program integration.
- Input and output mints MUST have active token policies for the battle's arena.
- Exact-input orders are the initial supported order type.
- Every swap MUST define a minimum output amount.
- Every swap MUST satisfy position, liquidity and price-impact limits.
- Deposits and withdrawals after battle start MUST fail.
- Trading costs count toward player performance.

## 8. Arena policy

### Safe Arena

Safe Arena accepts mature, liquid assets with conservative token-program behaviour.

### Meme Arena

Meme Arena accepts higher-risk assets only when they have a routable exit and satisfy battle-size-relative liquidity limits.

### Initially unsupported token behaviour

- Rebase accounting
- Unknown or unsafe transfer hooks
- Non-deterministic transfer taxes
- Frozen destination accounts
- Tokens that cannot be sold back to USDC or SOL
- Extensions the program cannot safely validate or settle

## 9. Settlement

Settlement is based on realised USDC, not an arbitrary last-traded oracle price.

Required properties:

- New risk MUST NOT be opened after trading lock.
- Remaining supported assets are converted to USDC.
- The same mint held by both players SHOULD use batch or equivalent pro-rata settlement to reduce ordering bias.
- Settlement operations MUST be idempotent.
- A failed route MUST NOT corrupt previously settled balances.
- Resolution MUST NOT occur until every tracked asset has a terminal outcome.

Winner calculation:

```text
player_a_equity = player_a_final_usdc
player_b_equity = player_b_final_usdc
gross_pool = player_a_equity + player_b_equity
protocol_fee = floor(gross_pool * fee_bps / 10_000)
winner_payout = gross_pool - protocol_fee
```

If the equities fall within the configured draw tolerance, the draw policy is used instead of winner-take-all. The draw tolerance and fee treatment remain open decisions.

## 10. Failure and emergency behaviour

The protocol must define a terminal result for:

- Opponent never joins
- Joined battle never starts
- Both players hold only USDC
- One player has zero equity
- Both players have zero equity
- A tracked mint loses all liquidity
- Jupiter or RPC providers are unavailable
- A token account becomes frozen
- Settlement partially completes
- Keeper disappears

Emergency actions must preserve player ownership and must not give the protocol multisig discretionary winner-selection authority.

## 11. Current implementation boundary

Version 0.2 code implements:

- Protocol configuration state
- Battle creation
- Opponent joining
- Start and time-derived lifecycle transitions
- Deterministic final-equity comparison
- Pure TypeScript reference model and tests
- Wallet Standard discovery and devnet connection UX
- Protocol-pinned six-decimal settlement mint
- Equal challenger and opponent SPL-token deposits
- Separate self-authorised PDA token vaults scoped by battle and trader
- Challenger refund when cancelling an open battle
- LiteSVM custody tests for equal deposits, isolated vaults, atomic refund, wrong mint and insufficient balance

Version 0.2 does not implement:

- Jupiter CPI
- Devnet program deployment and live web transaction submission
- Token policy accounts
- Settlement keeper execution
- Prize claims
- Mainnet-ready upgrade governance

## 12. Open decisions

- Stake tiers and limits
- Settlement fee
- Trading-lock duration
- Draw tolerance and refund fee
- Maximum distinct mints per player
- Safe and Meme Arena liquidity thresholds
- Unsellable-token terminal valuation
- Position visibility to the opponent
- Settlement gas sponsorship
- Start deadline and abandoned-battle timeout
