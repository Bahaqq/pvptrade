export const BATTLE_STATUS = {
  OPEN: "open",
  FUNDED: "funded",
  ACTIVE: "active",
  TRADING_LOCKED: "trading_locked",
  SETTLING: "settling",
  RESOLVED: "resolved",
  CLAIMED: "claimed",
  CANCELLED: "cancelled",
  REFUNDED: "refunded",
} as const;

export type BattleStatus = (typeof BATTLE_STATUS)[keyof typeof BATTLE_STATUS];

export const ARENA_TYPE = {
  SAFE: "safe",
  MEME: "meme",
} as const;

export type ArenaType = (typeof ARENA_TYPE)[keyof typeof ARENA_TYPE];

export type PublicKeyString = string;

export interface BattleTerms {
  id: string;
  challenger: PublicKeyString;
  settlementMint: PublicKeyString;
  stakeMicroUsdc: bigint;
  durationSeconds: number;
  tradingLockSeconds: number;
  settlementFeeBps: number;
  arena: ArenaType;
  createdAt: number;
}

export interface BattleOutcome {
  playerAFinalMicroUsdc: bigint;
  playerBFinalMicroUsdc: bigint;
  grossPoolMicroUsdc: bigint;
  protocolFeeMicroUsdc: bigint;
  payoutMicroUsdc: bigint;
  winner: PublicKeyString | null;
  isDraw: boolean;
}

export interface Battle {
  terms: Readonly<BattleTerms>;
  opponent: PublicKeyString | null;
  status: BattleStatus;
  startsAt: number | null;
  tradingLocksAt: number | null;
  tradingEndsAt: number | null;
  outcome: Readonly<BattleOutcome> | null;
  custody: Readonly<BattleCustody>;
}

export interface BattleCustody {
  challengerDepositedMicroUsdc: bigint;
  opponentDepositedMicroUsdc: bigint;
  challengerRefundedMicroUsdc: bigint;
}

export class BattleRuleError extends Error {
  constructor(
    public readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = "BattleRuleError";
  }
}

const MAX_BPS = 10_000;

function requireRule(condition: boolean, code: string, message: string): asserts condition {
  if (!condition) {
    throw new BattleRuleError(code, message);
  }
}

function requireStatus(battle: Battle, expected: BattleStatus): void {
  requireRule(
    battle.status === expected,
    "INVALID_STATUS",
    `Expected battle status ${expected}, received ${battle.status}.`,
  );
}

export function createBattle(terms: BattleTerms): Battle {
  requireRule(terms.id.trim().length > 0, "INVALID_ID", "Battle id is required.");
  requireRule(
    terms.challenger.trim().length > 0,
    "INVALID_CHALLENGER",
    "Challenger public key is required.",
  );
  requireRule(
    terms.settlementMint.trim().length > 0,
    "INVALID_SETTLEMENT_MINT",
    "Settlement mint public key is required.",
  );
  requireRule(terms.stakeMicroUsdc > 0n, "INVALID_STAKE", "Stake must be greater than zero.");
  requireRule(terms.durationSeconds > 0, "INVALID_DURATION", "Duration must be positive.");
  requireRule(
    Number.isInteger(terms.durationSeconds),
    "INVALID_DURATION",
    "Duration must be a whole number of seconds.",
  );
  requireRule(
    terms.tradingLockSeconds >= 0 && terms.tradingLockSeconds <= terms.durationSeconds,
    "INVALID_TRADING_LOCK",
    "Trading lock must be between zero and the battle duration.",
  );
  requireRule(
    Number.isInteger(terms.settlementFeeBps) &&
      terms.settlementFeeBps >= 0 &&
      terms.settlementFeeBps <= MAX_BPS,
    "INVALID_FEE",
    "Settlement fee must be an integer between 0 and 10,000 bps.",
  );

  return {
    terms: Object.freeze({ ...terms }),
    opponent: null,
    status: BATTLE_STATUS.OPEN,
    startsAt: null,
    tradingLocksAt: null,
    tradingEndsAt: null,
    outcome: null,
    custody: Object.freeze({
      challengerDepositedMicroUsdc: terms.stakeMicroUsdc,
      opponentDepositedMicroUsdc: 0n,
      challengerRefundedMicroUsdc: 0n,
    }),
  };
}

export function joinBattle(battle: Battle, opponent: PublicKeyString): Battle {
  requireStatus(battle, BATTLE_STATUS.OPEN);
  requireRule(opponent.trim().length > 0, "INVALID_OPPONENT", "Opponent public key is required.");
  requireRule(
    opponent !== battle.terms.challenger,
    "SELF_MATCH",
    "Challenger cannot join their own battle.",
  );

  return {
    ...battle,
    opponent,
    status: BATTLE_STATUS.FUNDED,
    custody: Object.freeze({
      ...battle.custody,
      opponentDepositedMicroUsdc: battle.terms.stakeMicroUsdc,
    }),
  };
}

export function startBattle(battle: Battle, now: number): Battle {
  requireStatus(battle, BATTLE_STATUS.FUNDED);
  requireRule(battle.opponent !== null, "MISSING_OPPONENT", "Battle must have an opponent.");
  requireRule(Number.isInteger(now), "INVALID_TIME", "Program time must be whole seconds.");

  const tradingEndsAt = now + battle.terms.durationSeconds;
  const tradingLocksAt = tradingEndsAt - battle.terms.tradingLockSeconds;

  return {
    ...battle,
    status: BATTLE_STATUS.ACTIVE,
    startsAt: now,
    tradingLocksAt,
    tradingEndsAt,
  };
}

export function lockTrading(battle: Battle, now: number): Battle {
  requireStatus(battle, BATTLE_STATUS.ACTIVE);
  requireRule(
    battle.tradingLocksAt !== null && now >= battle.tradingLocksAt,
    "TRADING_STILL_ACTIVE",
    "Trading cannot lock before the configured cutoff.",
  );

  return {
    ...battle,
    status: BATTLE_STATUS.TRADING_LOCKED,
  };
}

export function beginSettlement(battle: Battle, now: number): Battle {
  requireStatus(battle, BATTLE_STATUS.TRADING_LOCKED);
  requireRule(
    battle.tradingEndsAt !== null && now >= battle.tradingEndsAt,
    "BATTLE_NOT_ENDED",
    "Settlement cannot begin before trading ends.",
  );

  return {
    ...battle,
    status: BATTLE_STATUS.SETTLING,
  };
}

export interface ResolveBattleInput {
  playerAFinalMicroUsdc: bigint;
  playerBFinalMicroUsdc: bigint;
  drawToleranceMicroUsdc?: bigint;
}

export function resolveBattle(battle: Battle, input: ResolveBattleInput): Battle {
  requireStatus(battle, BATTLE_STATUS.SETTLING);
  requireRule(battle.opponent !== null, "MISSING_OPPONENT", "Battle must have an opponent.");
  requireRule(
    input.playerAFinalMicroUsdc >= 0n && input.playerBFinalMicroUsdc >= 0n,
    "NEGATIVE_EQUITY",
    "Final equity cannot be negative.",
  );

  const tolerance = input.drawToleranceMicroUsdc ?? 0n;
  requireRule(tolerance >= 0n, "INVALID_TOLERANCE", "Draw tolerance cannot be negative.");

  const grossPoolMicroUsdc = input.playerAFinalMicroUsdc + input.playerBFinalMicroUsdc;
  const protocolFeeMicroUsdc =
    (grossPoolMicroUsdc * BigInt(battle.terms.settlementFeeBps)) / BigInt(MAX_BPS);
  const payoutMicroUsdc = grossPoolMicroUsdc - protocolFeeMicroUsdc;
  const difference =
    input.playerAFinalMicroUsdc >= input.playerBFinalMicroUsdc
      ? input.playerAFinalMicroUsdc - input.playerBFinalMicroUsdc
      : input.playerBFinalMicroUsdc - input.playerAFinalMicroUsdc;
  const isDraw = difference <= tolerance;

  let winner: PublicKeyString | null = null;
  if (!isDraw) {
    winner =
      input.playerAFinalMicroUsdc > input.playerBFinalMicroUsdc
        ? battle.terms.challenger
        : battle.opponent;
  }

  return {
    ...battle,
    status: BATTLE_STATUS.RESOLVED,
    outcome: Object.freeze({
      playerAFinalMicroUsdc: input.playerAFinalMicroUsdc,
      playerBFinalMicroUsdc: input.playerBFinalMicroUsdc,
      grossPoolMicroUsdc,
      protocolFeeMicroUsdc,
      payoutMicroUsdc,
      winner,
      isDraw,
    }),
  };
}

export function cancelBattle(battle: Battle, actor: PublicKeyString): Battle {
  requireStatus(battle, BATTLE_STATUS.OPEN);
  requireRule(
    actor === battle.terms.challenger,
    "UNAUTHORISED",
    "Only the challenger can cancel an open battle.",
  );

  return {
    ...battle,
    status: BATTLE_STATUS.CANCELLED,
    custody: Object.freeze({
      ...battle.custody,
      challengerDepositedMicroUsdc: 0n,
      challengerRefundedMicroUsdc: battle.terms.stakeMicroUsdc,
    }),
  };
}

export function formatMicroUsdc(value: bigint): string {
  const sign = value < 0n ? "-" : "";
  const absolute = value < 0n ? -value : value;
  const whole = absolute / 1_000_000n;
  const fraction = (absolute % 1_000_000n).toString().padStart(6, "0").replace(/0+$/, "");
  return `${sign}${whole.toLocaleString("en-US")}${fraction ? `.${fraction}` : ""} USDC`;
}
