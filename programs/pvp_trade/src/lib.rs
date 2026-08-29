use anchor_lang::prelude::*;

// Lifecycle proof only. A deployment keypair and permanent program id are assigned
// before devnet deployment; the System Program id must never be used for deployment.
declare_id!("11111111111111111111111111111111");

const MAX_PROTOCOL_FEE_BPS: u16 = 1_000;

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
        config.max_settlement_fee_bps = max_settlement_fee_bps;
        config.default_trading_lock_seconds = default_trading_lock_seconds;
        config.paused = false;
        config.bump = ctx.bumps.protocol_config;

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

        emit!(BattleCreated {
            battle: battle.key(),
            battle_id,
            challenger: battle.challenger,
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
        battle.status = BattleStatus::Funded;

        emit!(BattleJoined {
            battle: battle.key(),
            opponent: battle.opponent,
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
        let battle = &mut ctx.accounts.battle;
        require!(battle.status == BattleStatus::Open, PvpTradeError::InvalidStatus);
        battle.status = BattleStatus::Cancelled;
        Ok(())
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
    #[account(mut)]
    pub challenger: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct JoinBattle<'info> {
    #[account(seeds = [b"protocol"], bump = protocol_config.bump)]
    pub protocol_config: Account<'info, ProtocolConfig>,
    #[account(mut, seeds = [b"battle", battle.id.as_ref()], bump = battle.bump)]
    pub battle: Account<'info, Battle>,
    pub opponent: Signer<'info>,
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
}

#[account]
pub struct ProtocolConfig {
    pub authority: Pubkey,
    pub max_settlement_fee_bps: u16,
    pub default_trading_lock_seconds: i64,
    pub paused: bool,
    pub bump: u8,
}

impl ProtocolConfig {
    pub const SPACE: usize = 8 + 32 + 2 + 8 + 1 + 1;
}

#[account]
pub struct Battle {
    pub id: [u8; 32],
    pub challenger: Pubkey,
    pub opponent: Pubkey,
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
}

impl Battle {
    pub const SPACE: usize = 8 + 256;
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
    pub stake_micro_usdc: u64,
    pub arena: Arena,
}

#[event]
pub struct BattleJoined {
    pub battle: Pubkey,
    pub opponent: Pubkey,
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
}
