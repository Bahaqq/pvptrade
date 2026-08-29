# PVP Trade

PVP Trade is a Solana-native competitive trading protocol. Two players deposit equal USDC stakes, trade from isolated battle vaults for a fixed period, and the player with the higher final equity wins the remaining pool after transparent protocol fees.

The project is currently in its protocol-design and proof-of-architecture phase. No contract in this repository is ready for real funds.

## Current scope

- Solana and native USDC
- 1v1 spot trading battles
- Jupiter-routed swaps
- Program-controlled PDA vaults
- Safe and meme-token arenas with separate risk policies
- Windows-first developer experience

## Repository layout

```text
apps/web/                  Next.js product shell
packages/protocol/         Executable battle domain model and tests
programs/pvp_trade/        Anchor program source
docs/                      Protocol and architecture specifications
scripts/                   PowerShell developer commands
ROADMAP.md                 Product and delivery roadmap
```

## Windows quick start

```powershell
.\scripts\setup.ps1
.\scripts\dev.ps1
```

Run all Windows-compatible checks:

```powershell
.\scripts\check.ps1
```

Create or inspect the local deployment keypair without overwriting an existing key:

```powershell
npm run program:keygen
```

The private program keypair stays under the ignored `.anchor/` directory. Back it up securely before devnet deployment; only the public program address belongs in Git.

The Anchor build is intentionally handled by cloud CI because Anchor's supported local Windows setup requires WSL. Contributors using this repository do not need to open a Linux terminal.

## Safety status

- The current Anchor program models lifecycle state only.
- Equal-stake SPL-token deposits into isolated battle PDA vaults are implemented.
- Open-battle cancellation returns the challenger's locked stake through a PDA-signed transfer.
- LiteSVM integration tests exercise real SPL Token CPI custody flows in cloud CI.
- Jupiter CPI, settlement payouts and prize claims are not implemented yet.
- The web app connects Wallet Standard wallets on devnet, but live battle transactions remain gated until deployment.
- Do not deploy the current program with real funds.
- Mainnet deployment requires completed threat modelling, independent audit, and explicit release approval.

## Documentation

- [Product roadmap](./ROADMAP.md)
- [Battle Protocol Specification](./docs/BATTLE_PROTOCOL_SPEC.md)
- [Architecture](./docs/ARCHITECTURE.md)
- [Architecture decisions](./docs/decisions/README.md)
