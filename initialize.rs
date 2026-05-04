use anchor_lang::prelude::*;
use crate::state::Config;
use crate::errors::ClawteryError;

/// Accounts required for the initialize instruction.
/// Creates the global Config PDA that stores game parameters.
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// The admin wallet that initializes the program.
    /// Must sign and pays for the Config PDA creation.
    #[account(mut)]
    pub admin: Signer<'info>,
    
    /// The global Config account.
    /// PDA derived from seed: "config".
    /// Stores coordinator, operations, entry cost, and current round.
    #[account(
        init,
        payer = admin,
        space = 8 + Config::INIT_SPACE,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, Config>,
    
    pub system_program: Program<'info, System>,
}

/// Initialize the Clawtery program.
///
/// # Arguments
/// * `coordinator` — Wallet that will execute draws and receive 10% share
/// * `operations` — Wallet that receives 2% operations share
///
/// # Errors
/// * `ClawteryError::Unauthorized` — if called by non-admin (enforced by program owner)
///
/// # Effects
/// Creates Config PDA with:
/// - entry_cost = 8_800_000 lamports (0.0088 SOL)
/// - winner_pct = 88, coordinator_pct = 10, operations_pct = 2
/// - current_round = 0
pub fn handler(
    ctx: Context<Initialize>,
    coordinator: Pubkey,
    operations: Pubkey,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    
    config.admin = ctx.accounts.admin.key();
    config.coordinator = coordinator;
    config.operations = operations;
    config.entry_cost = 8_800_000; // 0.0088 SOL in lamports
    config.winner_pct = 88;
    config.coordinator_pct = 10;
    config.operations_pct = 2;
    config.current_round = 0;
    config.bump = ctx.bumps.config;
    
    msg!("Clawtery initialized. Admin: {}, Coordinator: {}", config.admin, config.coordinator);
    Ok(())
}
