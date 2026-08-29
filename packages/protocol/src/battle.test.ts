import { describe, expect, it } from "vitest";
import {
  ARENA_TYPE,
  BATTLE_STATUS,
  BattleRuleError,
  beginSettlement,
  cancelBattle,
  createBattle,
  formatMicroUsdc,
  joinBattle,
  lockTrading,
  resolveBattle,
  startBattle,
} from "./battle";

const baseTerms = {
  id: "battle-001",
  challenger: "alice",
  settlementMint: "mock-usdc-mint",
  stakeMicroUsdc: 100_000_000n,
  durationSeconds: 86_400,
  tradingLockSeconds: 300,
  settlementFeeBps: 200,
  arena: ARENA_TYPE.MEME,
  createdAt: 1_000,
} as const;

function activeBattle() {
  return startBattle(joinBattle(createBattle(baseTerms), "bob"), 2_000);
}

describe("battle lifecycle", () => {
  it("moves through the happy path and pays the higher-equity player", () => {
    const active = activeBattle();
    expect(active.status).toBe(BATTLE_STATUS.ACTIVE);
    expect(active.custody).toEqual({
      challengerDepositedMicroUsdc: 100_000_000n,
      opponentDepositedMicroUsdc: 100_000_000n,
      challengerRefundedMicroUsdc: 0n,
    });
    expect(active.tradingLocksAt).toBe(88_100);
    expect(active.tradingEndsAt).toBe(88_400);

    const locked = lockTrading(active, 88_100);
    const settling = beginSettlement(locked, 88_400);
    const resolved = resolveBattle(settling, {
      playerAFinalMicroUsdc: 130_000_000n,
      playerBFinalMicroUsdc: 80_000_000n,
    });

    expect(resolved.status).toBe(BATTLE_STATUS.RESOLVED);
    expect(resolved.outcome).toEqual({
      playerAFinalMicroUsdc: 130_000_000n,
      playerBFinalMicroUsdc: 80_000_000n,
      grossPoolMicroUsdc: 210_000_000n,
      protocolFeeMicroUsdc: 4_200_000n,
      payoutMicroUsdc: 205_800_000n,
      winner: "alice",
      isDraw: false,
    });
  });

  it("rejects self matching", () => {
    expect(() => joinBattle(createBattle(baseTerms), "alice")).toThrowError(
      expect.objectContaining<Partial<BattleRuleError>>({ code: "SELF_MATCH" }),
    );
  });

  it("rejects an early trading lock", () => {
    expect(() => lockTrading(activeBattle(), 88_099)).toThrowError(
      expect.objectContaining<Partial<BattleRuleError>>({ code: "TRADING_STILL_ACTIVE" }),
    );
  });

  it("rejects settlement before the battle end", () => {
    const locked = lockTrading(activeBattle(), 88_100);
    expect(() => beginSettlement(locked, 88_399)).toThrowError(
      expect.objectContaining<Partial<BattleRuleError>>({ code: "BATTLE_NOT_ENDED" }),
    );
  });

  it("uses draw tolerance without selecting a winner", () => {
    const settling = beginSettlement(lockTrading(activeBattle(), 88_100), 88_400);
    const resolved = resolveBattle(settling, {
      playerAFinalMicroUsdc: 100_000_010n,
      playerBFinalMicroUsdc: 100_000_000n,
      drawToleranceMicroUsdc: 10n,
    });

    expect(resolved.outcome?.isDraw).toBe(true);
    expect(resolved.outcome?.winner).toBeNull();
  });

  it("only allows the challenger to cancel an open battle", () => {
    const open = createBattle(baseTerms);
    const cancelled = cancelBattle(open, "alice");
    expect(cancelled.status).toBe(BATTLE_STATUS.CANCELLED);
    expect(cancelled.custody).toEqual({
      challengerDepositedMicroUsdc: 0n,
      opponentDepositedMicroUsdc: 0n,
      challengerRefundedMicroUsdc: 100_000_000n,
    });
    expect(() => cancelBattle(open, "bob")).toThrowError(
      expect.objectContaining<Partial<BattleRuleError>>({ code: "UNAUTHORISED" }),
    );
  });
});

describe("battle validation", () => {
  it("requires a positive stake", () => {
    expect(() => createBattle({ ...baseTerms, stakeMicroUsdc: 0n })).toThrowError(
      expect.objectContaining<Partial<BattleRuleError>>({ code: "INVALID_STAKE" }),
    );
  });

  it("does not allow a lock window longer than the battle", () => {
    expect(() => createBattle({ ...baseTerms, tradingLockSeconds: 86_401 })).toThrowError(
      expect.objectContaining<Partial<BattleRuleError>>({ code: "INVALID_TRADING_LOCK" }),
    );
  });

  it("formats USDC base units", () => {
    expect(formatMicroUsdc(123_456_000n)).toBe("123.456 USDC");
    expect(formatMicroUsdc(100_000_000n)).toBe("100 USDC");
  });
});
