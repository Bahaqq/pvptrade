use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};

declare_id!("DwA1q1BU2Mf4B9GQ7PWcQ1TiPHA5rhkR2et1Xr8CRzww");

#[program]
pub mod mock_swap {
    use super::*;

    pub fn swap(ctx: Context<Swap>, amount_in: u64, amount_out: u64) -> Result<()> {
        require!(
            amount_in > 0 && amount_out > 0,
            MockSwapError::InvalidAmount
        );

        token::transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                TransferChecked {
                    from: ctx.accounts.source.to_account_info(),
                    mint: ctx.accounts.input_mint.to_account_info(),
                    to: ctx.accounts.pool_input.to_account_info(),
                    authority: ctx.accounts.source_authority.to_account_info(),
                },
            ),
            amount_in,
            ctx.accounts.input_mint.decimals,
        )?;

        let pool_bump = ctx.bumps.pool_authority;
        let signer_seeds: &[&[&[u8]]] = &[&[b"pool", &[pool_bump]]];
        token::transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                TransferChecked {
                    from: ctx.accounts.pool_output.to_account_info(),
                    mint: ctx.accounts.output_mint.to_account_info(),
                    to: ctx.accounts.destination.to_account_info(),
                    authority: ctx.accounts.pool_authority.to_account_info(),
                },
            )
            .with_signer(signer_seeds),
            amount_out,
            ctx.accounts.output_mint.decimals,
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut, token::mint = input_mint, token::authority = source_authority)]
    pub source: Account<'info, TokenAccount>,
    #[account(mut, token::mint = output_mint)]
    pub destination: Account<'info, TokenAccount>,
    #[account(mut, token::mint = input_mint)]
    pub pool_input: Account<'info, TokenAccount>,
    #[account(mut, token::mint = output_mint, token::authority = pool_authority)]
    pub pool_output: Account<'info, TokenAccount>,
    pub input_mint: Account<'info, Mint>,
    pub output_mint: Account<'info, Mint>,
    pub source_authority: Signer<'info>,
    /// CHECK: The seed constraint fixes the mock liquidity authority.
    #[account(seeds = [b"pool"], bump)]
    pub pool_authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

#[error_code]
pub enum MockSwapError {
    #[msg("Mock swap amounts must be positive.")]
    InvalidAmount,
}
