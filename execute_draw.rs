use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::slot_hashes::SlotHashes;
use crate::state::{Config, Round, RoundStatus, Entry};
use crate::errors::ClawteryError;
use sha2::{Sha256, Digest};

/// Accounts required for the execute_draw instruction.
/// Reads blockhashes, computes winner, distributes coordinator/operations shares.
#[derive(Accounts)]
#[instruction(round_number: u64)]
pub struct ExecuteDraw<'info> {
    /// Any caller can trigger draw execution (permissionless after slot reached).
    #[account(mut)]
    pub caller: Signer<'info>,
    
    /// The global Config account (read-only for percentages).
    #[account(
        seeds = [b"config"],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,
    
    /// The Round PDA (mutated to store winning number and results).
    #[account(
        mut,
        seeds = [b"round", round_number.to_le_bytes().as_ref()],
        bump = round.bump,
        constraint = round.status == RoundStatus::Committed @ ClawteryError::RoundNotCommitted,
    )]
    pub round: Account<'info, Round>,
    
    /// CHECK: Coordination wallet — receives 10% share.
    #[account(mut, address = config.coordinator)]
    pub coord_wallet: AccountInfo<'info>,
    
    /// CHECK: Operations wallet — receives 2% share.
    #[account(mut, address = config.operations)]
    pub ops_wallet: AccountInfo<'info>,
    
    /// CHECK: SlotHashes sysvar — provides verifiable on-chain randomness.
    #[account(address = anchor_lang::solana_program::sysvar::slot_hashes::id())]
    pub slot_hashes: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
}

/// Execute the draw and distribute shares.
///
/// # Randomness Source
/// Reads blockhashes from SlotHashes sysvar at 3 slots:
/// - draw_slot
/// - draw_slot + 2
/// - draw_slot + 4
///
/// Winning number = SHA256(hash1 || hash2 || hash3 || merkle_root)[0..8] as u64 LE
///
/// # Payout Distribution
/// - 88% → winner pool (distributed via claim_winnings)
/// - 10% → coordinator wallet (sent immediately)
/// - 2% → operations wallet (sent immediately)
///
/// # Remaining Accounts
/// Must pass ALL Entry PDAs for this round as remaining_accounts
/// in the same order they were created (index 0, 1, 2, ...).
///
/// # Errors
/// * `ClawteryError::SlotNotReached` — if draw_slot + 4 blocks not yet passed
/// * `ClawteryError::SlotHashesUnavailable` — if SlotHashes sysvar unreadable
/// * `ClawteryError::SlotNotFound` — if target slots not in sysvar
/// * `ClawteryError::NoEntries` — if round has zero entries
/// * `ClawteryError::InvalidEntry` — if remaining_accounts don't match entries
/// * `ClawteryError::NoWinners` — if no valid winner found
pub fn handler(
    ctx: Context<ExecuteDraw>,
    _round_number: u64,
) -> Result<()> {
    let round = &mut ctx.accounts.round;
    let config = &ctx.accounts.config;
    let clock = Clock::get()?;

    // Verify enough slots have passed since committed draw slot
    // We need draw_slot + 4 to guarantee all 3 hashes are available
    require!(
        clock.slot >= round.draw_slot + 4,
        ClawteryError::SlotNotReached
    );

    // Parse SlotHashes sysvar into a searchable data structure
    let slot_hashes = SlotHashes::from_account_info(&ctx.accounts.slot_hashes)
        .map_err(|_| ClawteryError::SlotHashesUnavailable)?;

    // Fetch blockhashes for the 3 draw positions
    let (slot_1, hash_1) = find_blockhash(&slot_hashes, round.draw_slot)
        .ok_or(ClawteryError::SlotNotFound)?;
    let (slot_2, hash_2) = find_blockhash(&slot_hashes, round.draw_slot + 2)
        .ok_or(ClawteryError::SlotNotFound)?;
    let (slot_3, hash_3) = find_blockhash(&slot_hashes, round.draw_slot + 4)
        .ok_or(ClawteryError::SlotNotFound)?;

    msg!("Draw slots: {}, {}, {}", slot_1, slot_2, slot_3);

    // Compute winning seed using SHA-256 of concatenated inputs
    let mut hasher = Sha256::new();
    hasher.update(hash_1.as_ref());
    hasher.update(hash_2.as_ref());
    hasher.update(hash_3.as_ref());
    hasher.update(round.merkle_root);
    let hash_result = hasher.finalize();

    // Convert first 8 bytes to u64 little-endian
    let winning_number = u64::from_le_bytes([
        hash_result[0], hash_result[1], hash_result[2], hash_result[3],
        hash_result[4], hash_result[5], hash_result[6], hash_result[7],
    ]);

    round.winning_number = winning_number;
    msg!("Winning number: {}", winning_number);

    // Require at least one entry to proceed
    require!(round.entry_count > 0, ClawteryError::NoEntries);

    // Validate remaining_accounts count matches entry_count
    require!(
        ctx.remaining_accounts.len() as u32 == round.entry_count,
        ClawteryError::InvalidEntry
    );

    // Parse all entry predictions from remaining accounts
    // We use manual byte parsing to avoid lifetime issues with Account::try_from
    let mut entries: Vec<(Pubkey, u64)> = Vec::with_capacity(ctx.remaining_accounts.len());
    for account in ctx.remaining_accounts.iter() {
        let data = account.try_borrow_data()?;
        
        // Validate minimum data length for Entry account
        require!(data.len() >= 60, ClawteryError::InvalidEntry);
        
        // Entry account layout (after 8-byte discriminator):
        // offset 0-7:   round (u64)
        // offset 8-11:  index (u32)
        // offset 12-43: owner (Pubkey, 32 bytes)
        // offset 44-51: prediction (u64)
        let entry_round = u64::from_le_bytes(data[8..16].try_into().unwrap());
        require_eq!(entry_round, round.round_number, ClawteryError::InvalidEntry);
        
        let owner = Pubkey::new_from_array(data[20..52].try_into().unwrap());
        let prediction = u64::from_le_bytes(data[52..60].try_into().unwrap());
        entries.push((owner, prediction));
    }

    // Calculate distances and find minimum
    let mut min_distance = u64::MAX;
    for (_, prediction) in &entries {
        let distance = prediction.abs_diff(winning_number);
        if distance < min_distance {
            min_distance = distance;
        }
    }

    msg!("Minimum distance: {}", min_distance);

    // Identify all winners (entries matching the minimum distance)
    let winners: Vec<Pubkey> = entries.iter()
        .filter(|(_, prediction)| {
            prediction.abs_diff(winning_number) == min_distance
        })
        .map(|(owner, _)| *owner)
        .collect();

    let winner_count = winners.len() as u32;
    require!(winner_count > 0, ClawteryError::NoWinners);

    msg!("Winner count: {}", winner_count);

    // Calculate payout shares using checked arithmetic to prevent overflow
    let total_pool = round.total_pool;
    let winner_share = (total_pool as u128)
        .checked_mul(config.winner_pct as u128)
        .ok_or(ClawteryError::InvalidEntry)?
        .checked_div(100)
        .ok_or(ClawteryError::InvalidEntry)? as u64;
    
    let coord_share = (total_pool as u128)
        .checked_mul(config.coordinator_pct as u128)
        .ok_or(ClawteryError::InvalidEntry)?
        .checked_div(100)
        .ok_or(ClawteryError::InvalidEntry)? as u64;
    
    let ops_share = total_pool
        .checked_sub(winner_share)
        .ok_or(ClawteryError::InvalidEntry)?
        .checked_sub(coord_share)
        .ok_or(ClawteryError::InvalidEntry)?;
    
    let per_winner = winner_share
        .checked_div(winner_count as u64)
        .ok_or(ClawteryError::NoWinners)?;

    // Store results in round account
    round.winner_count = winner_count;
    round.winner_share = winner_share;
    round.per_winner_share = per_winner;
    round.min_distance = min_distance;

    msg!("Winner share: {}, Coord: {}, Ops: {}", winner_share, coord_share, ops_share);

    // PDA signer seeds for the round account (required for CPI transfers FROM round)
    let round_number_bytes = round.round_number.to_le_bytes();
    let round_seeds = &[
        b"round".as_ref(),
        round_number_bytes.as_ref(),
        &[round.bump],
    ];
    let signer = &[&round_seeds[..]];

    // Transfer 10% coordinator share via CPI with PDA signing
    let cpi_accounts = anchor_lang::system_program::Transfer {
        from: round.to_account_info(),
        to: ctx.accounts.coord_wallet.to_account_info(),
    };
    let cpi_program = ctx.accounts.system_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    anchor_lang::system_program::transfer(cpi_ctx, coord_share)?;

    // Transfer 2% operations share via CPI with PDA signing
    let cpi_accounts = anchor_lang::system_program::Transfer {
        from: round.to_account_info(),
        to: ctx.accounts.ops_wallet.to_account_info(),
    };
    let cpi_program = ctx.accounts.system_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    anchor_lang::system_program::transfer(cpi_ctx, ops_share)?;

    // Mark round as drawn — winners can now claim
    round.status = RoundStatus::Drawn;

    msg!("Draw complete for round {}. {} winners must claim their share.", 
        round.round_number, winner_count);
    Ok(())
}

/// Search SlotHashes for a blockhash at or after target_slot.
/// Checks up to 10 slots forward to handle slot skipping.
fn find_blockhash(
    slot_hashes: &SlotHashes, 
    target_slot: u64
) -> Option<(u64, anchor_lang::solana_program::hash::Hash)> {
    for slot in target_slot..target_slot + 10 {
        if let Some(hash) = slot_hashes.get(&slot) {
            return Some((slot, *hash));
        }
    }
    None
}
