# PVP Trade Architecture

## System boundary

PVP Trade separates authoritative custody/state from replaceable user-experience services.

```text
Wallets
  |
  v
Next.js application ----> Quote/API coordinator ----> Jupiter APIs
  |                              |
  |                              v
  +----------------------> Solana RPC/indexer
  |
  v
PVP Trade program ----CPI----> Jupiter program ----> Solana DEX programs
  |
  +---- Battle PDA
  +---- Player A vault PDA
  +---- Player B vault PDA
  +---- Token policy PDAs
  +---- Fee vault PDA
```

## Trust model

### Authoritative on-chain components

- Battle terms and lifecycle
- Player identities and PDA authorities
- Allowed program/account validation
- Token-policy snapshots
- Settlement records and winner
- Protocol fee calculation

### Non-authoritative off-chain components

- Jupiter quote retrieval
- Cached PnL display
- Lobby and search indexes
- Notifications
- Keeper scheduling

If all off-chain services stop, funds must remain safe and a replacement client/keeper must be able to continue valid protocol actions.

## Repository architecture

### `packages/protocol`

Pure TypeScript reference implementation of battle rules. It provides fast Windows-native tests and a readable oracle for expected state transitions. It does not custody funds.

### `programs/pvp_trade`

Anchor source for the authoritative Solana program. Version 0.2 adds equal SPL-token stake deposits into self-authorised per-trader PDA vaults and a PDA-signed cancellation refund. The settlement mint is pinned in `ProtocolConfig` and must use six decimal places. Jupiter CPI remains isolated from this custody slice.

### `apps/web`

Product shell and battle UX. It uses the official `@solana/kit` Wallet Standard flow on devnet, exposes battle preparation controls, and consumes the protocol package for display types. Transaction submission remains disabled until a permanent program ID is deployed.

## Data flow for a future swap

1. Web app requests a Jupiter quote for the player's battle vault.
2. Risk service checks route, mint policy, position limit and price impact.
3. Client presents deterministic minimum output and fees.
4. Player signs a PVP Trade instruction.
5. PVP Trade validates state, signer, mints, accounts and destination.
6. PVP Trade invokes Jupiter via CPI using the player's vault PDA.
7. Output returns to the player's battle token account.
8. Program emits an event; indexer updates the read model.

## Windows-first delivery

- Frontend, reference model, linting and tests run with Node.js from PowerShell.
- PowerShell scripts are the supported local entrypoints.
- Anchor source is compiled and tested in cloud CI using pinned toolchain versions.
- Fast program experiments may use Solana Playground.
- Mainnet artifacts require reproducible builds and manual release approval.

## Security progression

1. Pure lifecycle model and negative-transition tests
2. Anchor lifecycle and account-constraint tests
3. Mock token custody and adversarial transfer tests (in progress; deposit/refund constraints implemented)
4. Jupiter CPI with strict destination validation
5. Settlement idempotency and partial-failure tests
6. Property/fuzz testing
7. Independent audit and closed mainnet alpha
