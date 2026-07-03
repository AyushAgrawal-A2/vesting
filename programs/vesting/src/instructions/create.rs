use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{self, Mint, TokenAccount, TokenInterface},
};

use crate::{constants::*, error::VestingError, state::Vesting};

#[derive(Accounts)]
#[instruction(id: u64)]
pub struct Create<'info> {
    #[account(mut)]
    creator: Signer<'info>,

    /// CHECK: allow system accounts or pda
    beneficiary: UncheckedAccount<'info>,

    mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = creator,
        associated_token::token_program = token_program
    )]
    creator_token_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = creator,
        space = 8 + Vesting::INIT_SPACE,
        seeds = [VESTING_SEED, creator.key().as_ref(), beneficiary.key().as_ref(), mint.key().as_ref(), id.to_le_bytes().as_ref()],
        bump
    )]
    vesting: Account<'info, Vesting>,

    #[account(
        init_if_needed,
        payer = creator,
        associated_token::mint = mint,
        associated_token::authority = vesting,
        associated_token::token_program = token_program
    )]
    vesting_vault: InterfaceAccount<'info, TokenAccount>,

    associated_token_program: Program<'info, AssociatedToken>,
    token_program: Interface<'info, TokenInterface>,
    system_program: Program<'info, System>,
}

pub fn handle_create(
    ctx: Context<Create>,
    id: u64,
    amount: u64,
    start: i64,
    cliff: i64,
    duration: u64,
) -> Result<()> {
    require!(amount > 0, VestingError::InvalidArguments);
    require!(cliff >= start, VestingError::InvalidArguments);
    require!(
        cliff
            <= start
                .checked_add(duration as i64)
                .ok_or(VestingError::Overflow)?,
        VestingError::InvalidArguments
    );
    require!(duration > 0, VestingError::InvalidArguments);

    token_interface::transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            token_interface::TransferChecked {
                from: ctx.accounts.creator_token_ata.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.vesting_vault.to_account_info(),
                authority: ctx.accounts.creator.to_account_info(),
            },
        ),
        amount,
        ctx.accounts.mint.decimals,
    )?;

    ctx.accounts.vesting_vault.reload()?;

    ctx.accounts.vesting.set_inner(Vesting {
        id,
        creator: ctx.accounts.creator.key(),
        beneficiary: ctx.accounts.beneficiary.key(),
        mint: ctx.accounts.mint.key(),
        total_amount: ctx.accounts.vesting_vault.amount,
        total_claimed: 0,
        start,
        cliff,
        duration,
        bump: ctx.bumps.vesting,
    });

    Ok(())
}
