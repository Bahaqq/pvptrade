use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
};
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};

declare_id!("2oQvjoTFEP8pyxhNgtSH7aCpoVfQK7wcWoQSLcQqE3wF");

const MAX_PROTOCOL_FEE_BPS: u16 = 1_000;
const MAX_SWAP_SLIPPAGE_BPS: u16 = 2_000;
const MAX_ROUTE_DATA_BYTES: usize = 1_024;

#[program]
pub mod pvp_trade {
    use super::*;

    pub fn initialize_protocol(
        ctx: Context<InitializeProtocol>,
        max_settlement_fee_bps: u16,
        default_trading_lock_seconds: i64,
    ) -> Result<()> {
        require!(
            max_settlement_fee_bps <= MAX_PROTOCOL_FEE_BPS,
            PvpTradeError::FeeTooHigh
        );
        require!(
            default_trading_lock_seconds >= 0,
            PvpTradeError::InvalidTradingLock
        );

        let config = &mut ctx.accounts.protocol_config;
        config.authority = ctx.accounts.authority.key();
        config.settlement_mint = ctx.accounts.settlement_mint.key();
        config.max_settlement_fee_bps = max_settlement_fee_bps;
        config.default_trading_lock_seconds = default_trading_lock_seconds;
        config.paused = false;
        config.bump = ctx.bumps.protocol_config;

        Ok(())
    }

    pub fn initialize_swap_config(
        ctx: Context<InitializeSwapConfig>,
        max_slippage_bps: u16,
    ) -> Result<()> {
        require!(
            max_slippage_bps > 0 && max_slippage_bps <= MAX_SWAP_SLIPPAGE_BPS,
            PvpTradeError::InvalidSlippage
        );

        let config = &mut ctx.accounts.swap_config;
        config.approved_swap_program = ctx.accounts.swap_program.key();
        config.max_slippage_bps = max_slippage_bps;
        config.bump = ctx.bumps.swap_config;
        Ok(())
    }

    pub fn create_token_policy(
        ctx: Context<CreateTokenPolicy>,
        safe_enabled: bool,
        meme_enabled: bool,
    ) -> Result<()> {
        require!(safe_enabled || meme_enabled, PvpTradeError::TokenNotAllowed);

        let policy = &mut ctx.accounts.token_policy;
        policy.mint = ctx.accounts.mint.key();
        policy.safe_enabled = safe_enabled;
        policy.meme_enabled = meme_enabled;
        policy.bump = ctx.bumps.token_policy;
        Ok(())
    }

    pub fn create_battle(
        ctx: Context<CreateBattle>,
        battle_id: [u8; 32],
        stake_micro_usdc: u64,
        duration_seconds: i64,
        trading_lock_seconds: i64,
        settlement_fee_bps: u16,
        arena: Arena,
    ) -> Result<()> {
        let config = &ctx.accounts.protocol_config;
        require!(!config.paused, PvpTradeError::ProtocolPaused);
        require!(stake_micro_usdc > 0, PvpTradeError::InvalidStake);
        require!(duration_seconds > 0, PvpTradeError::InvalidDuration);
        require!(
            trading_lock_seconds >= 0 && trading_lock_seconds <= duration_seconds,
            PvpTradeError::InvalidTradingLock
        );
        require!(
            settlement_fee_bps <= config.max_settlement_fee_bps,
            PvpTradeError::FeeTooHigh
        );

        let battle = &mut ctx.accounts.battle;
        battle.id = battle_id;
        battle.challenger = ctx.accounts.challenger.key();
        battle.opponent = Pubkey::default();
        battle.settlement_mint = ctx.accounts.settlement_mint.key();
        battle.challenger_vault = ctx.accounts.challenger_vault.key();
        battle.opponent_vault = Pubkey::default();
        battle.stake_micro_usdc = stake_micro_usdc;
        battle.duration_seconds = duration_seconds;
        battle.trading_lock_seconds = trading_lock_seconds;
        battle.settlement_fee_bps = settlement_fee_bps;
        battle.arena = arena;
        battle.status = BattleStatus::Open;
        battle.created_at = Clock::get()?.unix_timestamp;
        battle.starts_at = 0;
        battle.trading_locks_at = 0;
        battle.trading_ends_at = 0;
        battle.player_a_final_micro_usdc = 0;
        battle.player_b_final_micro_usdc = 0;
        battle.winner = Pubkey::default();
        battle.is_draw = false;
        battle.bump = ctx.bumps.battle;
        battle.challenger_vault_bump = ctx.bumps.challenger_vault;
        battle.opponent_vault_bump = 0;

        token::transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                TransferChecked {
                    from: ctx.accounts.challenger_source.to_account_info(),
                    mint: ctx.accounts.settlement_mint.to_account_info(),
                    to: ctx.accounts.challenger_vault.to_account_info(),
                    authority: ctx.accounts.challenger.to_account_info(),
                },
            ),
            stake_micro_usdc,
            ctx.accounts.settlement_mint.decimals,
        )?;

        emit!(BattleCreated {
            battle: battle.key(),
            battle_id,
            challenger: battle.challenger,
            challenger_vault: battle.challenger_vault,
            stake_micro_usdc,
            arena,
        });

        Ok(())
    }

    pub fn join_battle(ctx: Context<JoinBattle>) -> Result<()> {
        let battle = &mut ctx.accounts.battle;
        require!(!ctx.accounts.protocol_config.paused, PvpTradeError::ProtocolPaused);
        require!(battle.status == BattleStatus::Open, PvpTradeError::InvalidStatus);
        require!(
            ctx.accounts.opponent.key() != battle.challenger,
            PvpTradeError::SelfMatch
        );

        battle.opponent = ctx.accounts.opponent.key();
        battle.opponent_vault = ctx.accounts.opponent_vault.key();
        battle.opponent_vault_bump = ctx.bumps.opponent_vault;

        token::transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                TransferChecked {
                    from: ctx.accounts.opponent_source.to_account_info(),
                    mint: ctx.accounts.settlement_mint.to_account_info(),
                    to: ctx.accounts.opponent_vault.to_account_info(),
                    authority: ctx.accounts.opponent.to_account_info(),
                },
            ),
            battle.stake_micro_usdc,
            ctx.accounts.settlement_mint.decimals,
        )?;

        battle.status = BattleStatus::Funded;

        emit!(BattleJoined {
            battle: battle.key(),
            opponent: battle.opponent,
            opponent_vault: battle.opponent_vault,
        });

        Ok(())
    }

    pub fn start_battle(ctx: Context<BattleActor>) -> Result<()> {
        let battle = &mut ctx.accounts.battle;
        require!(!ctx.accounts.protocol_config.paused, PvpTradeError::ProtocolPaused);
        require!(battle.status == BattleStatus::Funded, PvpTradeError::InvalidStatus);
        require!(
            ctx.accounts.actor.key() == battle.challenger
                || ctx.accounts.actor.key() == battle.opponent,
            PvpTradeError::Unauthorized
        );

        let now = Clock::get()?.unix_timestamp;
        let trading_ends_at = now
            .checked_add(battle.duration_seconds)
            .ok_or(PvpTradeError::ArithmeticOverflow)?;
        let trading_locks_at = trading_ends_at
            .checked_sub(battle.trading_lock_seconds)
            .ok_or(PvpTradeError::ArithmeticOverflow)?;

        battle.starts_at = now;
        battle.trading_locks_at = trading_locks_at;
        battle.trading_ends_at = trading_ends_at;
        battle.status = BattleStatus::Active;

        emit!(BattleStarted {
            battle: battle.key(),
            starts_at: now,
            trading_locks_at,
            trading_ends_at,
        });

        Ok(())
    }

    pub fn create_asset_vault(ctx: Context<CreateAssetVault>) -> Result<()> {
        validate_active_trader(
            &ctx.accounts.battle,
            &ctx.accounts.actor.key(),
            Clock::get()?.unix_timestamp,
        )?;
        require!(
            ctx.accounts.token_policy.allows(ctx.accounts.battle.arena),
            PvpTradeError::TokenNotAllowed
        );
        require!(
            ctx.accounts.mint.key() != ctx.accounts.battle.settlement_mint,
            PvpTradeError::InvalidSwapMint
        );
        Ok(())
    }

    pub fn execute_swap<'info>(
        ctx: Context<'_, '_, '_, 'info, ExecuteSwap<'info>>,
        amount_in: u64,
        minimum_amount_out: u64,
        quoted_amount_out: u64,
        slippage_bps: u16,
        route_data: Vec<u8>,
    ) -> Result<()> {
        require!(amount_in > 0 && minimum_amount_out > 0, PvpTradeError::InvalidSwapAmount);
        require!(
            quoted_amount_out >= minimum_amount_out,
            PvpTradeError::InvalidMinimumOutput
        );
        require!(
            slippage_bps <= ctx.accounts.swap_config.max_slippage_bps,
            PvpTradeError::InvalidSlippage
        );
        require!(
            route_data.len() <= MAX_ROUTE_DATA_BYTES,
            PvpTradeError::RouteDataTooLarge
        );

        let battle = &ctx.accounts.battle;
        let actor = ctx.accounts.actor.key();
        validate_active_trader(battle, &actor, Clock::get()?.unix_timestamp)?;
        require!(
            ctx.accounts.input_policy.allows(battle.arena)
                && ctx.accounts.output_policy.allows(battle.arena),
            PvpTradeError::TokenNotAllowed
        );
        require!(
            ctx.accounts.input_mint.key() != ctx.accounts.output_mint.key(),
            PvpTradeError::InvalidSwapMint
        );

        let input_mint = ctx.accounts.input_mint.key();
        let output_mint = ctx.accounts.output_mint.key();
        let battle_key = battle.key();
        let (expected_input, input_bump) = expected_player_vault(battle, &battle_key, &actor, &input_mint);
        let (expected_output, _) = expected_player_vault(battle, &battle_key, &actor, &output_mint);
        require_keys_eq!(ctx.accounts.input_vault.key(), expected_input, PvpTradeError::InvalidInputVault);
        require_keys_eq!(ctx.accounts.output_vault.key(), expected_output, PvpTradeError::InvalidOutputVault);
        require!(
            ctx.accounts.input_vault.amount >= amount_in,
            PvpTradeError::InsufficientVaultBalance
        );

        let route_has_input = ctx
            .remaining_accounts
            .iter()
            .any(|account| account.key() == ctx.accounts.input_vault.key());
        let route_has_output = ctx
            .remaining_accounts
            .iter()
            .any(|account| account.key() == ctx.accounts.output_vault.key());
        require!(route_has_input && route_has_output, PvpTradeError::InvalidRouteAccounts);
        require!(
            ctx.remaining_accounts.iter().all(|account| !account.is_signer),
            PvpTradeError::UnexpectedRouteSigner
        );

        let input_before = ctx.accounts.input_vault.amount;
        let output_before = ctx.accounts.output_vault.amount;
        let input_key = ctx.accounts.input_vault.key();
        let metas = ctx
            .remaining_accounts
            .iter()
            .map(|account| {
                if account.is_writable {
                    AccountMeta::new(*account.key, account.key() == input_key)
                } else {
                    AccountMeta::new_readonly(*account.key, account.key() == input_key)
                }
            })
            .collect();
        let account_infos = ctx.remaining_accounts.to_vec();
        let instruction = Instruction {
            program_id: ctx.accounts.swap_program.key(),
            accounts: metas,
            data: route_data,
        };
        let input_bump_seed = [input_bump];
        let signer_seeds: &[&[u8]] = if input_mint == battle.settlement_mint {
            &[b"vault", battle_key.as_ref(), actor.as_ref(), &input_bump_seed]
        } else {
            &[
                b"asset_vault",
                battle_key.as_ref(),
                actor.as_ref(),
                input_mint.as_ref(),
                &input_bump_seed,
            ]
        };
        invoke_signed(&instruction, &account_infos, &[signer_seeds])?;

        ctx.accounts.input_vault.reload()?;
        ctx.accounts.output_vault.reload()?;
        let spent = input_before
            .checked_sub(ctx.accounts.input_vault.amount)
            .ok_or(PvpTradeError::InvalidSwapDebit)?;
        let received = ctx
            .accounts
            .output_vault
            .amount
            .checked_sub(output_before)
            .ok_or(PvpTradeError::InvalidSwapCredit)?;
        require!(spent == amount_in, PvpTradeError::InvalidSwapDebit);
        require!(received >= minimum_amount_out, PvpTradeError::MinimumOutputNotMet);

        emit!(SwapExecuted {
            battle: battle_key,
            actor,
            input_mint,
            output_mint,
            amount_in: spent,
            amount_out: received,
        });
        Ok(())
    }

    pub fn lock_trading(ctx: Context<AdvanceBattle>) -> Result<()> {
        let battle = &mut ctx.accounts.battle;
        require!(battle.status == BattleStatus::Active, PvpTradeError::InvalidStatus);
        require!(
            Clock::get()?.unix_timestamp >= battle.trading_locks_at,
            PvpTradeError::TradingStillActive
        );

        battle.status = BattleStatus::TradingLocked;
        Ok(())
    }

    pub fn begin_settlement(ctx: Context<AdvanceBattle>) -> Result<()> {
        let battle = &mut ctx.accounts.battle;
        require!(
            battle.status == BattleStatus::TradingLocked,
            PvpTradeError::InvalidStatus
        );
        require!(
            Clock::get()?.unix_timestamp >= battle.trading_ends_at,
            PvpTradeError::BattleNotEnded
        );

        battle.status = BattleStatus::Settling;
        Ok(())
    }

    // Temporary lifecycle-proof instruction. Final equity inputs will be replaced by
    // verified settlement records before token custody is introduced.
    pub fn resolve_battle(
        ctx: Context<ResolveBattle>,
        player_a_final_micro_usdc: u64,
        player_b_final_micro_usdc: u64,
    ) -> Result<()> {
        let battle = &mut ctx.accounts.battle;
        require!(battle.status == BattleStatus::Settling, PvpTradeError::InvalidStatus);

        battle.player_a_final_micro_usdc = player_a_final_micro_usdc;
        battle.player_b_final_micro_usdc = player_b_final_micro_usdc;
        battle.is_draw = player_a_final_micro_usdc == player_b_final_micro_usdc;
        battle.winner = if battle.is_draw {
            Pubkey::default()
        } else if player_a_final_micro_usdc > player_b_final_micro_usdc {
            battle.challenger
        } else {
            battle.opponent
        };
        battle.status = BattleStatus::Resolved;

        emit!(BattleResolved {
            battle: battle.key(),
            player_a_final_micro_usdc,
            player_b_final_micro_usdc,
            winner: battle.winner,
            is_draw: battle.is_draw,
        });

        Ok(())
    }

    pub fn cancel_battle(ctx: Context<CancelBattle>) -> Result<()> {
        require!(
            ctx.accounts.battle.status == BattleStatus::Open,
            PvpTradeError::InvalidStatus
        );

        let battle_key = ctx.accounts.battle.key();
        let challenger_key = ctx.accounts.challenger.key();
        let challenger_vault_bump = ctx.accounts.battle.challenger_vault_bump;
        let stake_micro_usdc = ctx.accounts.battle.stake_micro_usdc;
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"vault",
            battle_key.as_ref(),
            challenger_key.as_ref(),
            &[challenger_vault_bump],
        ]];

        token::transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                TransferChecked {
                    from: ctx.accounts.challenger_vault.to_account_info(),
                    mint: ctx.accounts.settlement_mint.to_account_info(),
                    to: ctx.accounts.challenger_refund.to_account_info(),
                    authority: ctx.accounts.challenger_vault.to_account_info(),
                },
            )
            .with_signer(signer_seeds),
            stake_micro_usdc,
            ctx.accounts.settlement_mint.decimals,
        )?;

        ctx.accounts.battle.status = BattleStatus::Cancelled;
        Ok(())
    }
}

fn validate_active_trader(battle: &Account<Battle>, actor: &Pubkey, now: i64) -> Result<()> {
    require!(battle.status == BattleStatus::Active, PvpTradeError::InvalidStatus);
    require!(
        *actor == battle.challenger || *actor == battle.opponent,
        PvpTradeError::Unauthorized
    );
    require!(now < battle.trading_locks_at, PvpTradeError::TradingLocked);
    Ok(())
}

fn expected_player_vault(
    battle: &Battle,
    battle_key: &Pubkey,
    actor: &Pubkey,
    mint: &Pubkey,
) -> (Pubkey, u8) {
    if *mint == battle.settlement_mint {
        let (vault, bump) = Pubkey::find_program_address(
            &[b"vault", battle_key.as_ref(), actor.as_ref()],
            &crate::ID,
        );
        (vault, bump)
    } else {
        Pubkey::find_program_address(
            &[
                b"asset_vault",
                battle_key.as_ref(),
                actor.as_ref(),
                mint.as_ref(),
            ],
            &crate::ID,
        )
    }
}

#[derive(Accounts)]
pub struct InitializeProtocol<'info> {
    #[account(
        init,
        payer = authority,
        space = ProtocolConfig::SPACE,
        seeds = [b"protocol"],
        bump
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
    #[account(constraint = settlement_mint.decimals == 6 @ PvpTradeError::InvalidSettlementMintDecimals)]
    pub settlement_mint: Account<'info, Mint>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeSwapConfig<'info> {
    #[account(
        seeds = [b"protocol"],
        bump = protocol_config.bump,
        has_one = authority @ PvpTradeError::Unauthorized
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
    #[account(
        init,
        payer = authority,
        space = SwapConfig::SPACE,
        seeds = [b"swap_config"],
        bump
    )]
    pub swap_config: Account<'info, SwapConfig>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: Executability and the stored address constrain all future CPI calls.
    #[account(executable)]
    pub swap_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateTokenPolicy<'info> {
    #[account(
        seeds = [b"protocol"],
        bump = protocol_config.bump,
        has_one = authority @ PvpTradeError::Unauthorized
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
    #[account(
        init,
        payer = authority,
        space = TokenPolicy::SPACE,
        seeds = [b"token_policy", mint.key().as_ref()],
        bump
    )]
    pub token_policy: Account<'info, TokenPolicy>,
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(battle_id: [u8; 32])]
pub struct CreateBattle<'info> {
    #[account(seeds = [b"protocol"], bump = protocol_config.bump)]
    pub protocol_config: Account<'info, ProtocolConfig>,
    #[account(
        init,
        payer = challenger,
        space = Battle::SPACE,
        seeds = [b"battle", battle_id.as_ref()],
        bump
    )]
    pub battle: Account<'info, Battle>,
    #[account(address = protocol_config.settlement_mint @ PvpTradeError::InvalidSettlementMint)]
    pub settlement_mint: Account<'info, Mint>,
    #[account(
        mut,
        token::mint = settlement_mint,
        token::authority = challenger
    )]
    pub challenger_source: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = challenger,
        seeds = [b"vault", battle.key().as_ref(), challenger.key().as_ref()],
        bump,
        token::mint = settlement_mint,
        token::authority = challenger_vault
    )]
    pub challenger_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub challenger: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct JoinBattle<'info> {
    #[account(seeds = [b"protocol"], bump = protocol_config.bump)]
    pub protocol_config: Account<'info, ProtocolConfig>,
    #[account(mut, seeds = [b"battle", battle.id.as_ref()], bump = battle.bump)]
    pub battle: Account<'info, Battle>,
    #[account(address = battle.settlement_mint @ PvpTradeError::InvalidSettlementMint)]
    pub settlement_mint: Account<'info, Mint>,
    #[account(
        mut,
        token::mint = settlement_mint,
        token::authority = opponent
    )]
    pub opponent_source: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = opponent,
        seeds = [b"vault", battle.key().as_ref(), opponent.key().as_ref()],
        bump,
        token::mint = settlement_mint,
        token::authority = opponent_vault
    )]
    pub opponent_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub opponent: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BattleActor<'info> {
    #[account(seeds = [b"protocol"], bump = protocol_config.bump)]
    pub protocol_config: Account<'info, ProtocolConfig>,
    #[account(mut, seeds = [b"battle", battle.id.as_ref()], bump = battle.bump)]
    pub battle: Account<'info, Battle>,
    pub actor: Signer<'info>,
}

#[derive(Accounts)]
pub struct CreateAssetVault<'info> {
    #[account(mut, seeds = [b"battle", battle.id.as_ref()], bump = battle.bump)]
    pub battle: Account<'info, Battle>,
    #[account(
        seeds = [b"token_policy", mint.key().as_ref()],
        bump = token_policy.bump,
        has_one = mint @ PvpTradeError::InvalidSwapMint
    )]
    pub token_policy: Account<'info, TokenPolicy>,
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        payer = actor,
        seeds = [b"asset_vault", battle.key().as_ref(), actor.key().as_ref(), mint.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = asset_vault
    )]
    pub asset_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub actor: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExecuteSwap<'info> {
    #[account(seeds = [b"protocol"], bump = protocol_config.bump)]
    pub protocol_config: Account<'info, ProtocolConfig>,
    #[account(
        seeds = [b"swap_config"],
        bump = swap_config.bump,
        has_one = approved_swap_program @ PvpTradeError::InvalidSwapProgram
    )]
    pub swap_config: Account<'info, SwapConfig>,
    #[account(mut, seeds = [b"battle", battle.id.as_ref()], bump = battle.bump)]
    pub battle: Account<'info, Battle>,
    #[account(
        seeds = [b"token_policy", input_mint.key().as_ref()],
        bump = input_policy.bump,
        constraint = input_policy.mint == input_mint.key() @ PvpTradeError::InvalidSwapMint
    )]
    pub input_policy: Account<'info, TokenPolicy>,
    #[account(
        seeds = [b"token_policy", output_mint.key().as_ref()],
        bump = output_policy.bump,
        constraint = output_policy.mint == output_mint.key() @ PvpTradeError::InvalidSwapMint
    )]
    pub output_policy: Account<'info, TokenPolicy>,
    pub input_mint: Account<'info, Mint>,
    pub output_mint: Account<'info, Mint>,
    #[account(
        mut,
        token::mint = input_mint,
        token::authority = input_vault
    )]
    pub input_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        token::mint = output_mint,
        token::authority = output_vault
    )]
    pub output_vault: Account<'info, TokenAccount>,
    pub actor: Signer<'info>,
    /// CHECK: Its address is pinned by SwapConfig and it must remain executable.
    #[account(address = approved_swap_program, executable)]
    pub approved_swap_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct AdvanceBattle<'info> {
    #[account(mut, seeds = [b"battle", battle.id.as_ref()], bump = battle.bump)]
    pub battle: Account<'info, Battle>,
}

#[derive(Accounts)]
pub struct ResolveBattle<'info> {
    #[account(
        seeds = [b"protocol"],
        bump = protocol_config.bump,
        has_one = authority @ PvpTradeError::Unauthorized
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
    pub authority: Signer<'info>,
    #[account(mut, seeds = [b"battle", battle.id.as_ref()], bump = battle.bump)]
    pub battle: Account<'info, Battle>,
}

#[derive(Accounts)]
pub struct CancelBattle<'info> {
    #[account(
        mut,
        seeds = [b"battle", battle.id.as_ref()],
        bump = battle.bump,
        has_one = challenger @ PvpTradeError::Unauthorized
    )]
    pub battle: Account<'info, Battle>,
    pub challenger: Signer<'info>,
    #[account(address = battle.settlement_mint @ PvpTradeError::InvalidSettlementMint)]
    pub settlement_mint: Account<'info, Mint>,
    #[account(
        mut,
        address = battle.challenger_vault,
        token::mint = settlement_mint,
        token::authority = challenger_vault
    )]
    pub challenger_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        token::mint = settlement_mint,
        token::authority = challenger
    )]
    pub challenger_refund: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct ProtocolConfig {
    pub authority: Pubkey,
    pub settlement_mint: Pubkey,
    pub max_settlement_fee_bps: u16,
    pub default_trading_lock_seconds: i64,
    pub paused: bool,
    pub bump: u8,
}

impl ProtocolConfig {
    pub const SPACE: usize = 8 + 32 + 32 + 2 + 8 + 1 + 1;
}

#[account]
pub struct SwapConfig {
    pub approved_swap_program: Pubkey,
    pub max_slippage_bps: u16,
    pub bump: u8,
}

impl SwapConfig {
    pub const SPACE: usize = 8 + 32 + 2 + 1;
}

#[account]
pub struct TokenPolicy {
    pub mint: Pubkey,
    pub safe_enabled: bool,
    pub meme_enabled: bool,
    pub bump: u8,
}

impl TokenPolicy {
    pub const SPACE: usize = 8 + 32 + 1 + 1 + 1;

    pub fn allows(&self, arena: Arena) -> bool {
        match arena {
            Arena::Safe => self.safe_enabled,
            Arena::Meme => self.meme_enabled,
        }
    }
}

#[account]
pub struct Battle {
    pub id: [u8; 32],
    pub challenger: Pubkey,
    pub opponent: Pubkey,
    pub settlement_mint: Pubkey,
    pub challenger_vault: Pubkey,
    pub opponent_vault: Pubkey,
    pub stake_micro_usdc: u64,
    pub duration_seconds: i64,
    pub trading_lock_seconds: i64,
    pub settlement_fee_bps: u16,
    pub arena: Arena,
    pub status: BattleStatus,
    pub created_at: i64,
    pub starts_at: i64,
    pub trading_locks_at: i64,
    pub trading_ends_at: i64,
    pub player_a_final_micro_usdc: u64,
    pub player_b_final_micro_usdc: u64,
    pub winner: Pubkey,
    pub is_draw: bool,
    pub bump: u8,
    pub challenger_vault_bump: u8,
    pub opponent_vault_bump: u8,
}

impl Battle {
    pub const SPACE: usize = 8 + 384;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arena {
    Safe,
    Meme,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleStatus {
    Open,
    Funded,
    Active,
    TradingLocked,
    Settling,
    Resolved,
    Claimed,
    Cancelled,
    Refunded,
}

#[event]
pub struct BattleCreated {
    pub battle: Pubkey,
    pub battle_id: [u8; 32],
    pub challenger: Pubkey,
    pub challenger_vault: Pubkey,
    pub stake_micro_usdc: u64,
    pub arena: Arena,
}

#[event]
pub struct BattleJoined {
    pub battle: Pubkey,
    pub opponent: Pubkey,
    pub opponent_vault: Pubkey,
}

#[event]
pub struct BattleStarted {
    pub battle: Pubkey,
    pub starts_at: i64,
    pub trading_locks_at: i64,
    pub trading_ends_at: i64,
}

#[event]
pub struct BattleResolved {
    pub battle: Pubkey,
    pub player_a_final_micro_usdc: u64,
    pub player_b_final_micro_usdc: u64,
    pub winner: Pubkey,
    pub is_draw: bool,
}

#[event]
pub struct SwapExecuted {
    pub battle: Pubkey,
    pub actor: Pubkey,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
}

#[error_code]
pub enum PvpTradeError {
    #[msg("Protocol actions are paused.")]
    ProtocolPaused,
    #[msg("The battle is not in the required state.")]
    InvalidStatus,
    #[msg("Stake must be greater than zero.")]
    InvalidStake,
    #[msg("Battle duration must be positive.")]
    InvalidDuration,
    #[msg("Trading lock must be between zero and the battle duration.")]
    InvalidTradingLock,
    #[msg("Settlement fee exceeds the configured maximum.")]
    FeeTooHigh,
    #[msg("A challenger cannot join their own battle.")]
    SelfMatch,
    #[msg("The signer is not authorised for this action.")]
    Unauthorized,
    #[msg("Trading is still active.")]
    TradingStillActive,
    #[msg("The battle has not reached its end time.")]
    BattleNotEnded,
    #[msg("Arithmetic overflow while calculating battle time.")]
    ArithmeticOverflow,
    #[msg("Settlement mint does not match the protocol configuration.")]
    InvalidSettlementMint,
    #[msg("Settlement mint must use six decimal places.")]
    InvalidSettlementMintDecimals,
    #[msg("The configured swap slippage is outside the allowed range.")]
    InvalidSlippage,
    #[msg("The token is not enabled for this battle arena.")]
    TokenNotAllowed,
    #[msg("The swap mint pair is invalid.")]
    InvalidSwapMint,
    #[msg("Trading is locked for this battle.")]
    TradingLocked,
    #[msg("The swap amount must be positive.")]
    InvalidSwapAmount,
    #[msg("The minimum output exceeds the quoted output.")]
    InvalidMinimumOutput,
    #[msg("The router instruction data is too large.")]
    RouteDataTooLarge,
    #[msg("The configured swap program does not match.")]
    InvalidSwapProgram,
    #[msg("The input vault does not belong to this player and battle.")]
    InvalidInputVault,
    #[msg("The output vault does not belong to this player and battle.")]
    InvalidOutputVault,
    #[msg("The input vault does not contain enough tokens.")]
    InsufficientVaultBalance,
    #[msg("The route omits the required player vault accounts.")]
    InvalidRouteAccounts,
    #[msg("The route requested an unexpected transaction signer.")]
    UnexpectedRouteSigner,
    #[msg("The router did not debit the exact requested input amount.")]
    InvalidSwapDebit,
    #[msg("The router reduced the destination balance.")]
    InvalidSwapCredit,
    #[msg("The router returned less than the required minimum output.")]
    MinimumOutputNotMet,
}
