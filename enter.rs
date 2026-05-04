use anchor_lang::prelude::*;
use crate::state::{Config, Round, RoundStatus, Entry};
use crate::errors::ClawteryError;

/// Accounts required for the enter instruction.
/// Creates an Entry PDA and transfers entry cost to the round account.
#[derive(Accounts)]
#[instruction(round_number: u64, prediction: u64)]
pub struct Enter<'info> {
    /// The player submitting the prediction.
    /// Must sign and pay for Entry PDA creation + entry cost.
    #[account(mut)]
    pub player: Signer<'info>,
    
    /// The global Config account (read-only for entry cost).
    #[account(
        seeds = [b"config"],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,
    
    /// The Round PDA (mutated to track entries and pool).
    #[account(
        mut,
        seeds = [b"round", round_number.to_le_bytes().as_ref()],
        bump = round.bump,
        constraint = round.status == RoundStatus::Open @ ClawteryError::RoundNotOpen,
    )]
    pub round: Account<'info, Round>,
    
    /// The Entry PDA to create.
    /// PDA derived from seeds: ["entry", round_number, entry_index].
    #[account(
        init,
        payer = player,
        space = 8 + Entry::INIT_SPACE,
        seeds = [
            b"entry",
            round_number.to_le_bytes().as_ref(),
            round.entry_count.to_le_bytes().as_ref(),
        ],
        bump
    )]
    pub entry: Account<'info, Entry>,
    
    pub system_program: Program<'info, System>,
}

/// Submit a prediction for the active round.
///
/// # Arguments
/// * `_round_number` — Round being entered (validated by PDA seeds)
/// * `prediction` — The u64 hash prediction value
///
/// # Validation
/// * Round must be Open
/// * Current time must be before cutoff_time
/// * Entry cost (0.0088 SOL) is transferred from player to round PDA
///
/// # Errors
/// * `ClawteryError::RoundNotOpen` — if round status is not Open
/// * `ClawteryError::SubmissionClosed` — if past cutoff time
pub fn handler(
    ctx: Context<Enter>,
    _round_number: u64,
    prediction: u64,
) -> Result<()> {
    let config = &ctx.accounts.config;
    let round = &mut ctx.accounts.round;
    let entry = &mut ctx.accounts.entry;
    let clock = Clock::get()?;

    // Validate submission window: must submit before cutoff
    require!(
        clock.unix_timestamp < round.cutoff_time,
        ClawteryError::SubmissionClosed
    );

    // Transfer entry cost from player to round account via CPI
    let cpi_accounts = anchor_lang::system_program::Transfer {
        from: ctx.accounts.player.to_account_info(),
        to: round.to_account_info(),
    };
    let cpi_program = ctx.accounts.system_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    anchor_lang::system_program::transfer(cpi_ctx, config.entry_cost)?;

    // Initialize the Entry PDA
    entry.round = round.round_number;
    entry.index = round.entry_count;
    entry.owner = ctx.accounts.player.key();
    entry.prediction = prediction;
    entry.claimed = false;
    entry.bump = ctx.bumps.entry;

    // Update round totals
    round.entry_count += 1;
    round.total_pool += config.entry_cost;

    msg!("Entry {} submitted for round {}. Prediction: {}. Pool: {}",
        entry.index, round.round_number, prediction, round.total_pool);
    Ok(())
}
