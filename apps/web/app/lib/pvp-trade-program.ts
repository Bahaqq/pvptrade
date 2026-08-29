import {
  AccountRole,
  address,
  getAddressEncoder,
  getProgramDerivedAddress,
  type Address,
  type Instruction,
  type InstructionWithSigners,
  type TransactionSigner,
} from "@solana/kit";
import { findAssociatedTokenPda, TOKEN_PROGRAM_ADDRESS } from "@solana-program/token";
import { SOLANA_RPC_URL } from "./solana-client";

export const PVP_TRADE_PROGRAM_ADDRESS = address(
  "2oQvjoTFEP8pyxhNgtSH7aCpoVfQK7wcWoQSLcQqE3wF",
);
export const DEVNET_USDC_MINT_ADDRESS = address(
  "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU",
);
export const SYSTEM_PROGRAM_ADDRESS = address("11111111111111111111111111111111");

const CREATE_BATTLE_DISCRIMINATOR = new Uint8Array([2, 249, 54, 216, 42, 99, 187, 102]);
const JOIN_BATTLE_DISCRIMINATOR = new Uint8Array([126, 0, 69, 130, 127, 145, 54, 100]);
const textEncoder = new TextEncoder();
const addressEncoder = getAddressEncoder();

export type BattleArena = "safe" | "meme";

export interface CreateBattleInstructionInput {
  arena: BattleArena;
  battleId: string;
  challenger: TransactionSigner;
  durationSeconds?: bigint;
  settlementFeeBps?: number;
  stakeMicroUsdc: bigint;
  tradingLockSeconds?: bigint;
}

export interface JoinBattleInstructionInput {
  battleId: string;
  opponent: TransactionSigner;
}

export interface BattleAddresses {
  battle: Address;
  battleIdBytes: Uint8Array;
  protocolConfig: Address;
}

function encodeUnsigned(value: bigint, byteLength: number, label: string): Uint8Array {
  if (value < 0n) throw new Error(`${label} cannot be negative.`);
  const maximum = (1n << BigInt(byteLength * 8)) - 1n;
  if (value > maximum) throw new Error(`${label} exceeds its onchain integer range.`);

  const bytes = new Uint8Array(byteLength);
  const view = new DataView(bytes.buffer);
  if (byteLength === 8) view.setBigUint64(0, value, true);
  else if (byteLength === 2) view.setUint16(0, Number(value), true);
  else throw new Error(`Unsupported unsigned integer size: ${byteLength}.`);
  return bytes;
}

function encodeSigned(value: bigint, label: string): Uint8Array {
  const minimum = -(1n << 63n);
  const maximum = (1n << 63n) - 1n;
  if (value < minimum || value > maximum) throw new Error(`${label} exceeds the i64 range.`);

  const bytes = new Uint8Array(8);
  new DataView(bytes.buffer).setBigInt64(0, value, true);
  return bytes;
}

function concatBytes(...parts: Uint8Array[]): Uint8Array {
  const output = new Uint8Array(parts.reduce((total, part) => total + part.length, 0));
  let offset = 0;
  for (const part of parts) {
    output.set(part, offset);
    offset += part.length;
  }
  return output;
}

export function parseBattleId(value: string): Uint8Array {
  const normalized = value.trim().replace(/^#/, "").toLowerCase();
  if (!/^[0-9a-f]{64}$/.test(normalized)) {
    throw new Error("Battle ID must contain exactly 64 hexadecimal characters.");
  }

  return Uint8Array.from(normalized.match(/.{2}/g) ?? [], (pair) => Number.parseInt(pair, 16));
}

export function createRandomBattleId(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(32));
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

export function parseUsdcAmount(value: string): bigint {
  const normalized = value.trim();
  if (!/^\d+(?:\.\d{1,6})?$/.test(normalized)) {
    throw new Error("Use a positive USDC amount with at most 6 decimal places.");
  }

  const [whole, fraction = ""] = normalized.split(".");
  const amount = BigInt(whole) * 1_000_000n + BigInt(fraction.padEnd(6, "0"));
  if (amount <= 0n) throw new Error("Stake must be greater than zero.");
  return amount;
}

export async function deriveBattleAddresses(battleId: string): Promise<BattleAddresses> {
  const battleIdBytes = parseBattleId(battleId);
  const [[protocolConfig], [battle]] = await Promise.all([
    getProgramDerivedAddress({
      programAddress: PVP_TRADE_PROGRAM_ADDRESS,
      seeds: [textEncoder.encode("protocol")],
    }),
    getProgramDerivedAddress({
      programAddress: PVP_TRADE_PROGRAM_ADDRESS,
      seeds: [textEncoder.encode("battle"), battleIdBytes],
    }),
  ]);

  return { battle, battleIdBytes, protocolConfig };
}

async function derivePlayerVault(battle: Address, player: Address): Promise<Address> {
  const [vault] = await getProgramDerivedAddress({
    programAddress: PVP_TRADE_PROGRAM_ADDRESS,
    seeds: [textEncoder.encode("vault"), addressEncoder.encode(battle), addressEncoder.encode(player)],
  });
  return vault;
}

async function deriveUsdcAccount(owner: Address): Promise<Address> {
  const [tokenAccount] = await findAssociatedTokenPda({
    mint: DEVNET_USDC_MINT_ADDRESS,
    owner,
    tokenProgram: TOKEN_PROGRAM_ADDRESS,
  });
  return tokenAccount;
}

export async function getCreateBattleInstruction(
  input: CreateBattleInstructionInput,
): Promise<Instruction & InstructionWithSigners> {
  const durationSeconds = input.durationSeconds ?? 86_400n;
  const tradingLockSeconds = input.tradingLockSeconds ?? 300n;
  const settlementFeeBps = input.settlementFeeBps ?? 200;
  if (durationSeconds <= 0n) throw new Error("Battle duration must be positive.");
  if (tradingLockSeconds < 0n || tradingLockSeconds > durationSeconds) {
    throw new Error("Trading lock must be between zero and the battle duration.");
  }
  if (!Number.isInteger(settlementFeeBps) || settlementFeeBps < 0 || settlementFeeBps > 1_000) {
    throw new Error("Settlement fee must be between 0 and 1,000 basis points.");
  }

  const { battle, battleIdBytes, protocolConfig } = await deriveBattleAddresses(input.battleId);
  const [challengerSource, challengerVault] = await Promise.all([
    deriveUsdcAccount(input.challenger.address),
    derivePlayerVault(battle, input.challenger.address),
  ]);
  const data = concatBytes(
    CREATE_BATTLE_DISCRIMINATOR,
    battleIdBytes,
    encodeUnsigned(input.stakeMicroUsdc, 8, "Stake"),
    encodeSigned(durationSeconds, "Duration"),
    encodeSigned(tradingLockSeconds, "Trading lock"),
    encodeUnsigned(BigInt(settlementFeeBps), 2, "Settlement fee"),
    new Uint8Array([input.arena === "safe" ? 0 : 1]),
  );
  const accounts: InstructionWithSigners["accounts"] = [
    { address: protocolConfig, role: AccountRole.READONLY },
    { address: battle, role: AccountRole.WRITABLE },
    { address: DEVNET_USDC_MINT_ADDRESS, role: AccountRole.READONLY },
    { address: challengerSource, role: AccountRole.WRITABLE },
    { address: challengerVault, role: AccountRole.WRITABLE },
    { address: input.challenger.address, role: AccountRole.WRITABLE_SIGNER, signer: input.challenger },
    { address: TOKEN_PROGRAM_ADDRESS, role: AccountRole.READONLY },
    { address: SYSTEM_PROGRAM_ADDRESS, role: AccountRole.READONLY },
  ];

  return {
    programAddress: PVP_TRADE_PROGRAM_ADDRESS,
    accounts,
    data,
  };
}

export async function getJoinBattleInstruction(
  input: JoinBattleInstructionInput,
): Promise<Instruction & InstructionWithSigners> {
  const { battle, protocolConfig } = await deriveBattleAddresses(input.battleId);
  const [opponentSource, opponentVault] = await Promise.all([
    deriveUsdcAccount(input.opponent.address),
    derivePlayerVault(battle, input.opponent.address),
  ]);
  const accounts: InstructionWithSigners["accounts"] = [
    { address: protocolConfig, role: AccountRole.READONLY },
    { address: battle, role: AccountRole.WRITABLE },
    { address: DEVNET_USDC_MINT_ADDRESS, role: AccountRole.READONLY },
    { address: opponentSource, role: AccountRole.WRITABLE },
    { address: opponentVault, role: AccountRole.WRITABLE },
    { address: input.opponent.address, role: AccountRole.WRITABLE_SIGNER, signer: input.opponent },
    { address: TOKEN_PROGRAM_ADDRESS, role: AccountRole.READONLY },
    { address: SYSTEM_PROGRAM_ADDRESS, role: AccountRole.READONLY },
  ];

  return {
    programAddress: PVP_TRADE_PROGRAM_ADDRESS,
    accounts,
    data: JOIN_BATTLE_DISCRIMINATOR,
  };
}

export async function isPvpTradeProgramDeployed(): Promise<boolean> {
  const response = await fetch(SOLANA_RPC_URL, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "getAccountInfo",
      params: [PVP_TRADE_PROGRAM_ADDRESS, { commitment: "confirmed", encoding: "base64" }],
    }),
  });
  if (!response.ok) throw new Error(`Devnet RPC returned HTTP ${response.status}.`);

  const payload = (await response.json()) as {
    error?: { message?: string };
    result?: { value?: { executable?: boolean } | null };
  };
  if (payload.error) throw new Error(payload.error.message ?? "Devnet RPC request failed.");
  return payload.result?.value?.executable === true;
}
