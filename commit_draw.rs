use anchor_lang::prelude::*;
use crate::state::{Config, Round, RoundStatus};
use crate::errors::ClawteryError;

/// Accounts required for the commit_draw instruction.
/// Coordinator commits the draw parameters after entries close.
#[derive(Accounts)]
#[instruction(round_number: u64, merkle_root: [u8; 32], draw_slot: u64)]
pub struct CommitDraw<'info> {
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
    
    /// The Round PDA (mutated to store commitment).
    #[account(
        mut,
        seeds = [b"round", round_number.to_le_bytes().as_ref()],
        bump = round.bump,
        constraint = round.status == RoundStatus::Open @ ClawteryError::RoundNotOpen,
    )]
    pub round: Account<'info, Round>,
}

/// Coordinator commits the draw after cutoff time.
///
/// # Arguments
/// * `_round_number` — Round being committed (validated by PDA)
/// * `merkle_root` — Commitment hash (mixes with blockhashes for randomness)
/// * `draw_slot` — Slot number from which blockhashes will be read
///
/// # Validation
/// * Only coordinator can commit
/// * Must be after cutoff_time (entries closed)
/// * draw_slot must be in the future (not yet produced)
///
/// # Errors
/// * `ClawteryError::Unauthorized` — if signer is not coordinator
/// * `ClawteryError::RoundNotOpen` — if round not in Open status
/// * `ClawteryError::CommitTooEarly` — if before cutoff time
/// * `ClawteryError::SlotNotReached` — if draw_slot is not in the future
pub fn handler(
    ctx: Context<CommitDraw>,
    _round_number: u64,
    merkle_root: [u8; 32],
    draw_slot: u64,
) -> Result<()> {
    let round = &mut ctx.accounts.round;
    let clock = Clock::get()?;

    // Can only commit after cutoff time ensures no new entries can be added
    require!(
        clock.unix_timestamp >= round.cutoff_time,
        ClawteryError::CommitTooEarly
    );

    // Draw slot must be in the future so blockhash is not yet known
    require!(
        draw_slot > clock.slot,
        ClawteryError::SlotNotReached
    );

    // Store commitment and advance round status
    round.merkle_root = merkle_root;
    round.draw_slot = draw_slot;
    round.status = RoundStatus::Committed;

    msg!("Round {} committed. Draw slot: {}. Merkle root set.",
        round.round_number, draw_slot);
    Ok(())
}
