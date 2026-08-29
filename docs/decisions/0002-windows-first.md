# ADR 0002: Windows-first development

## Status

Accepted — 29 August 2026

## Decision

Contributors must be able to run the supported application workflow from PowerShell without opening WSL or a Linux terminal.

## Consequences

- Node.js application checks run natively on Windows.
- Local entrypoints are `.ps1` scripts.
- Anchor builds use cloud CI or browser tooling.
- Build internals may use a pinned remote environment, but Linux knowledge is not a contributor prerequisite.
