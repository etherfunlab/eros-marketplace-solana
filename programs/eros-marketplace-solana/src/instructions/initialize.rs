use anchor_lang::prelude::*;

use crate::seeds::PROGRAM_CONFIG_SEED;
use crate::state::ProgramConfig;

/// One-shot bootstrap of the program-wide `ProgramConfig` PDA. The admin
/// pubkey is captured here and gates all privileged instructions
/// (`init_registries`, `set_listing_quote`, `housekeeping_clear`).
///
/// The instruction is `init` (not `init_if_needed`) so a re-init attempt
/// fails. There is no admin-rotation path in v0.1.x.
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Must sign so the deploying operator demonstrably controls the admin
    /// key, not just submits a pubkey for someone else.
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + ProgramConfig::INIT_SPACE,
        seeds = [PROGRAM_CONFIG_SEED],
        bump,
    )]
    pub program_config: Account<'info, ProgramConfig>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Initialize>) -> Result<()> {
    let c = &mut ctx.accounts.program_config;
    c.admin = ctx.accounts.admin.key();
    c.bump = ctx.bumps.program_config;
    Ok(())
}
