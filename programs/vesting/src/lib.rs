pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("C6msbnfexiVBenZyabFQrDdvBAZXfkF3RDSEMyWX3Tzx");

#[program]
pub mod vesting {
    use super::*;

    pub fn create(
        ctx: Context<Create>,
        id: u64,
        amount: u64,
        start: i64,
        cliff: i64,
        duration: u64,
    ) -> Result<()> {
        crate::instructions::create::handle_create(ctx, id, amount, start, cliff, duration)
    }

    pub fn claim(ctx: Context<Claim>, id: u64) -> Result<()> {
        crate::instructions::claim::handle_claim(ctx, id)
    }
}
