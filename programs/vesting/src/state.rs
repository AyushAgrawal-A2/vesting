use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Vesting {
    pub id: u64,
    pub creator: Pubkey,
    pub beneficiary: Pubkey,
    pub mint: Pubkey,
    pub total_amount: u64,
    pub total_claimed: u64,
    pub start: i64,
    pub cliff: i64,
    pub duration: u64,
    pub bump: u8,
}
