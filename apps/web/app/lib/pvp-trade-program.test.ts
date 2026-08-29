import { AccountRole, generateKeyPairSigner } from "@solana/kit";
import { describe, expect, it } from "vitest";
import {
  DEVNET_USDC_MINT_ADDRESS,
  PVP_TRADE_PROGRAM_ADDRESS,
  deriveBattleAddresses,
  getCreateBattleInstruction,
  getJoinBattleInstruction,
  parseBattleId,
  parseUsdcAmount,
} from "./pvp-trade-program";

const BATTLE_ID = "ab".repeat(32);

describe("PVP Trade instruction client", () => {
  it("parses USDC without floating-point arithmetic", () => {
    expect(parseUsdcAmount("25")).toBe(25_000_000n);
    expect(parseUsdcAmount("0.000001")).toBe(1n);
    expect(() => parseUsdcAmount("1.0000001")).toThrow(/6 decimal/);
    expect(() => parseUsdcAmount("0")).toThrow(/greater than zero/);
  });

  it("requires a complete 32-byte battle id", () => {
    expect(parseBattleId(BATTLE_ID)).toEqual(new Uint8Array(32).fill(0xab));
    expect(() => parseBattleId("abcd")).toThrow(/64 hexadecimal/);
  });

  it("derives stable battle and protocol PDAs", async () => {
    await expect(deriveBattleAddresses(BATTLE_ID)).resolves.toEqual({
      battle: "DEMNiVgQ3hzazbkjtytsdnfKJUzn7Vz6R8CN5wg4a6rN",
      battleIdBytes: new Uint8Array(32).fill(0xab),
      protocolConfig: "GujFjDLTKhwG1srrUvuBCyRtNCpMZKqjQcXvLvyw1Ttt",
    });
  });

  it("encodes the Anchor create and join instructions with signer metadata", async () => {
    const signer = await generateKeyPairSigner();
    const createInstruction = await getCreateBattleInstruction({
      arena: "meme",
      battleId: BATTLE_ID,
      challenger: signer,
      stakeMicroUsdc: 25_000_000n,
    });
    const joinInstruction = await getJoinBattleInstruction({ battleId: BATTLE_ID, opponent: signer });

    expect(createInstruction.programAddress).toBe(PVP_TRADE_PROGRAM_ADDRESS);
    expect(createInstruction.accounts).toHaveLength(8);
    expect(createInstruction.accounts?.[2].address).toBe(DEVNET_USDC_MINT_ADDRESS);
    expect(createInstruction.accounts?.[5]).toMatchObject({
      address: signer.address,
      role: AccountRole.WRITABLE_SIGNER,
      signer,
    });
    expect(createInstruction.data).toHaveLength(67);
    expect(Array.from(createInstruction.data?.slice(0, 8) ?? [])).toEqual([2, 249, 54, 216, 42, 99, 187, 102]);
    expect(createInstruction.data?.[66]).toBe(1);

    expect(joinInstruction.accounts).toHaveLength(8);
    expect(Array.from(joinInstruction.data ?? [])).toEqual([126, 0, 69, 130, 127, 145, 54, 100]);
  });
});
