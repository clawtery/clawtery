use anchor_lang::prelude::*;
use crate::state::{Config, Round, RoundStatus};
use crate::errors::ClawteryError;

/// Accounts required for the cancel_empty instruction.
/// Coordinator cancels a round that received zero entries.
#[derive(Accounts)]
#[instruction(round_number: u64)]
pub struct CancelEmpty<'info> {
    /// The coordinator wallet. Must match config.coordinator.
    #[account(mut)]
    pub coordinator: Signer<'info>,
    
    /// The global Config account (read-only, validates coordinator).
    #[account(
        seeds = [b"config"],
        bump = config.bump,
        constraint = coordinator.key() == config.coordinator @ ClawteryError::Unauthorized,
    )]
    pub config: Account<'info, Config>,
    
    /// The Round PDA (mutated to Empty status).
    #[account(
        mut,
        seeds = [b"round", round_number.to_le_bytes().as_ref()],
        bump = round.bump,
        constraint = round.status == RoundStatus::Open @ ClawteryError::RoundNotOpen,
    )]
    pub round: Account<'info, Round>,
}

/// Cancel an empty round after cutoff.
///
/// # Use Case
/// If no agents entered the round, coordinator can cancel to free the round PDA
/// and allow starting the next round.
///
/// # Validation
/// * Only coordinator can cancel
/// * Must be after cutoff_time
/// * Round must have exactly 0 entries
/// * Round must be in Open status
///
/// # Errors
/// * `ClawteryError::Unauthorized` — if signer is not coordinator
/// * `ClawteryError::RoundNotOpen` — if round not in Open status
/// * `ClawteryError::CommitTooEarly` — if before cutoff time
/// * `ClawteryError::InvalidEntry` — if round has entries (cannot cancel)
pub fn handler(
    ctx: Context<CancelEmpty>,
    _round_number: u64,
) -> Result<()> {
    let round = &mut ctx.accounts.round;
    let clock = Clock::get()?;

    // Can only cancel after cutoff time (same as commit window)
    require!(
        clock.unix_timestamp >= round.cutoff_time,
        ClawteryError::CommitTooEarly
    );
    
    // Can only cancel if no entries were submitted
    require!(
        round.entry_count == 0,
        ClawteryError::InvalidEntry
    );

    // Mark round as empty/cancelled
    round.status = RoundStatus::Empty;
    
    msg!("Round {} cancelled (no entries)", round.round_number);
    Ok(())
}
