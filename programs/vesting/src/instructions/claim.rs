use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{self, Mint, TokenAccount, TokenInterface},
};

use crate::{constants::*, error::VestingError, state::Vesting};

#[derive(Accounts)]
#[instruction(id: u64)]
pub struct Claim<'info> {
    #[account(mut)]
    beneficiary: Signer<'info>,

    /// CHECK: allow system accounts or pda, checked in has_one constraint and part of seeds
    creator: UncheckedAccount<'info>,

    mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = beneficiary,
        associated_token::mint = mint,
        associated_token::authority = beneficiary,
        associated_token::token_program = token_program
    )]
    beneficiary_token_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        has_one = creator,
        has_one = beneficiary,
        has_one = mint,
        seeds = [VESTING_SEED, creator.key().as_ref(), beneficiary.key().as_ref(), mint.key().as_ref(), id.to_le_bytes().as_ref()],
        bump = vesting.bump
    )]
    vesting: Account<'info, Vesting>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = vesting,
        associated_token::token_program = token_program
    )]
    vesting_vault: InterfaceAccount<'info, TokenAccount>,

    associated_token_program: Program<'info, AssociatedToken>,
    token_program: Interface<'info, TokenInterface>,
    system_program: Program<'info, System>,
}

pub fn handle_claim(ctx: Context<Claim>, id: u64) -> Result<()> {
    let current_time = Clock::get()?.unix_timestamp;
    require!(
        current_time >= ctx.accounts.vesting.cliff,
        VestingError::InvalidClaimBeforeCliff
    );

    let vested = if current_time
        >= ctx
            .accounts
            .vesting
            .start
            .checked_add(ctx.accounts.vesting.duration as i64)
            .ok_or(VestingError::Overflow)?
    {
        ctx.accounts.vesting.total_amount
    } else {
        (ctx.accounts.vesting.total_amount as u128
            * ((current_time - ctx.accounts.vesting.start) as u128)
            / ctx.accounts.vesting.duration as u128) as u64
    };

    let amount = vested
        .checked_sub(ctx.accounts.vesting.total_claimed)
        .ok_or(VestingError::Underflow)?;
    require!(amount > 0, VestingError::InvalidClaimAmount);

    ctx.accounts.vesting.total_claimed = ctx
        .accounts
        .vesting
        .total_claimed
        .checked_add(amount)
        .ok_or(VestingError::Overflow)?;

    let creator_address = ctx.accounts.creator.key();
    let beneficiary_address = ctx.accounts.beneficiary.key();
    let mint_address = ctx.accounts.mint.key();
    let seeds = [
        VESTING_SEED,
        creator_address.as_ref(),
        beneficiary_address.as_ref(),
        mint_address.as_ref(),
        &id.to_le_bytes(),
        &[ctx.accounts.vesting.bump],
    ];
    token_interface::transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            token_interface::TransferChecked {
                from: ctx.accounts.vesting_vault.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.beneficiary_token_ata.to_account_info(),
                authority: ctx.accounts.vesting.to_account_info(),
            },
            &[&seeds[..]],
        ),
        amount,
        ctx.accounts.mint.decimals,
    )?;

    Ok(())
}
