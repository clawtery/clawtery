use anchor_lang::prelude::*;
use crate::state::{Config, Round, RoundStatus, Entry};
use crate::errors::ClawteryError;

/// Accounts required for the claim_winnings instruction.
/// Winner claims their share of the 88% prize pool.
#[derive(Accounts)]
#[instruction(round_number: u64, entry_index: u32)]
pub struct ClaimWinnings<'info> {
    /// The winner claiming their prize. Must be the entry owner.
    #[account(mut)]
    pub winner: Signer<'info>,
    
    /// The global Config account (read-only for validation).
    #[account(
        seeds = [b"config"],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,
    
    /// The Round PDA (mutated to track claims).
    #[account(
        mut,
        seeds = [b"round", round_number.to_le_bytes().as_ref()],
        bump = round.bump,
        constraint = round.status == RoundStatus::Drawn @ ClawteryError::RoundNotCommitted,
    )]
    pub round: Account<'info, Round>,
    
    /// The Entry PDA (mutated to mark claimed).
    /// Must match the round_number and entry_index.
    #[account(
        mut,
        seeds = [
            b"entry",
            round_number.to_le_bytes().as_ref(),
            entry_index.to_le_bytes().as_ref(),
        ],
        bump = entry.bump,
        constraint = entry.owner == winner.key() @ ClawteryError::Unauthorized,
        constraint = !entry.claimed @ ClawteryError::InvalidEntry,
    )]
    pub entry: Account<'info, Entry>,
    
    pub system_program: Program<'info, System>,
}

/// Claim winnings for a winning entry.
///
/// # Arguments
/// * `_round_number` — Round being claimed (validated by PDA)
/// * `_entry_index` — Entry index (validated by PDA)
///
/// # Validation
/// * Entry owner must match signer
/// * Entry must not already be claimed
/// * Entry's distance must equal round.min_distance
/// * Round must be in Drawn status
///
/// # Payout
/// Transfers round.per_winner_share lamports from round PDA to winner wallet.
///
/// # Errors
/// * `ClawteryError::Unauthorized` — if signer is not entry owner
/// * `ClawteryError::InvalidEntry` — if entry already claimed or invalid
/// * `ClawteryError::RoundNotCommitted` — if round not yet drawn
/// * `ClawteryError::NoWinners` — if entry is not a winner (distance != min)
pub fn handler(
    ctx: Context<ClaimWinnings>,
    _round_number: u64,
    _entry_index: u32,
) -> Result<()> {
    let round = &mut ctx.accounts.round;
    let entry = &mut ctx.accounts.entry;

    // Verify this entry is a winner by checking its distance against minimum
    let distance = entry.prediction.abs_diff(round.winning_number);
    require!(
        distance == round.min_distance,
        ClawteryError::NoWinners
    );

    // Verify there is a prize to claim
    let claim_amount = round.per_winner_share;
    require!(claim_amount > 0, ClawteryError::NoWinners);

    // Mark entry as claimed to prevent double-claiming
    entry.claimed = true;

    // Transfer prize from round PDA to winner via CPI with PDA signing
    let round_number_bytes = round.round_number.to_le_bytes();
    let round_seeds = &[
        b"round".as_ref(),
        round_number_bytes.as_ref(),
        &[round.bump],
    ];
    let signer = &[&round_seeds[..]];

    let cpi_accounts = anchor_lang::system_program::Transfer {
        from: round.to_account_info(),
        to: ctx.accounts.winner.to_account_info(),
    };
    let cpi_program = ctx.accounts.system_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    anchor_lang::system_program::transfer(cpi_ctx, claim_amount)?;

    msg!("Winner {} claimed {} lamports for round {}",
        ctx.accounts.winner.key(), claim_amount, round.round_number);
    Ok(())
}
