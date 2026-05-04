use anchor_lang::prelude::*;
use crate::state::{Config, Round, RoundStatus};
use crate::errors::ClawteryError;

/// Accounts required for the start_round instruction.
/// Creates a new Round PDA and increments the global round counter.
#[derive(Accounts)]
#[instruction(round_number: u64, draw_time: i64)]
pub struct StartRound<'info> {
    /// The admin wallet. Must match config.admin.
    #[account(mut)]
    pub admin: Signer<'info>,
    
    /// The global Config account (mutated to update current_round).
    #[account(
        mut,
        has_one = admin @ ClawteryError::Unauthorized,
        seeds = [b"config"],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,
    
    /// The Round PDA to create.
    /// PDA derived from seeds: ["round", round_number].
    #[account(
        init,
        payer = admin,
        space = 8 + Round::INIT_SPACE,
        seeds = [b"round", round_number.to_le_bytes().as_ref()],
        bump
    )]
    pub round: Account<'info, Round>,
    
    pub system_program: Program<'info, System>,
}

/// Start a new round.
///
/// # Arguments
/// * `round_number` — Sequential round ID (must be current_round + 1)
/// * `draw_time` — Unix timestamp when the draw will execute
///
/// # Validation
/// * Round number must be sequential
/// * Cutoff time is automatically set to draw_time - 360 seconds (6 minutes)
///
/// # Errors
/// * `ClawteryError::Unauthorized` — if signer is not config.admin
/// * `ClawteryError::InvalidRoundNumber` — if round_number != current_round + 1
pub fn handler(
    ctx: Context<StartRound>,
    round_number: u64,
    draw_time: i64,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    
    // Validate sequential round numbering to prevent skipping or replay
    require!(
        round_number == config.current_round + 1,
        ClawteryError::InvalidRoundNumber
    );
    
    // Validate draw_time is in the future
    let clock = Clock::get()?;
    require!(
        draw_time > clock.unix_timestamp,
        ClawteryError::DrawTimeNotReached
    );

    let round = &mut ctx.accounts.round;
    
    // Initialize round state
    round.round_number = round_number;
    round.draw_time = draw_time;
    round.cutoff_time = draw_time - 360; // 6 minutes before draw
    round.entry_count = 0;
    round.total_pool = 0;
    round.status = RoundStatus::Open;
    round.merkle_root = [0u8; 32];
    round.draw_slot = 0;
    round.winning_number = 0;
    round.winner_count = 0;
    round.winner_share = 0;
    round.per_winner_share = 0;
    round.min_distance = u64::MAX;
    round.bump = ctx.bumps.round;

    // Advance global round counter
    config.current_round = round_number;
    
    msg!("Round {} started. Draw at {}, cutoff at {}", 
        round_number, draw_time, round.cutoff_time);
    Ok(())
}
