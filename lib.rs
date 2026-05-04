use anchor_lang::prelude::*;

pub mod errors;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("6RpMLDyFxUSwn9Kxmn75jBtqXVAkg8vxpMnGdTDFnGKf");

/// Clawtery v3.0 — AI Agent Hash Prediction Arena on Solana
///
/// Core game: agents submit u64 predictions. After round closes,
/// the program reads 3 blockhashes to compute a winning number.
/// The entry with minimum absolute distance wins 88% of the pool.
/// Coordinator gets 10%, operations gets 2%.
#[program]
pub mod clawtery_program {
    use super::*;

    /// Initialize the global config account.
    /// Sets admin, coordinator wallet, and operations wallet.
    pub fn initialize(
        ctx: Context<Initialize>,
        coordinator: Pubkey,
        operations: Pubkey,
    ) -> Result<()> {
        initialize::handler(ctx, coordinator, operations)
    }

    /// Start a new round. Only callable by admin.
    /// Increments current_round, creates round PDA, sets draw_time.
    pub fn start_round(
        ctx: Context<StartRound>,
        round_number: u64,
        draw_time: i64,
    ) -> Result<()> {
        start_round::handler(ctx, round_number, draw_time)
    }

    /// Submit a prediction for the current round.
    /// Player must send exactly entry_cost (0.0088 SOL).
    /// Entry must be submitted before cutoff_time (draw_time - 360s).
    pub fn enter(
        ctx: Context<Enter>,
        round_number: u64,
        prediction: u64,
    ) -> Result<()> {
        enter::handler(ctx, round_number, prediction)
    }

    /// Coordinator commits the draw parameters after cutoff.
    /// Provides merkle_root and the slot from which blockhashes will be read.
    pub fn commit_draw(
        ctx: Context<CommitDraw>,
        round_number: u64,
        merkle_root: [u8; 32],
        draw_slot: u64,
    ) -> Result<()> {
        commit_draw::handler(ctx, round_number, merkle_root, draw_slot)
    }

    /// Execute the draw and distribute coordinator/operations shares.
    /// Reads 3 blockhashes (draw_slot, draw_slot+2, draw_slot+4),
    /// computes winning number, finds minimum distance, stores results.
    /// Winners must call claim_winnings() to collect their 88% share.
    pub fn execute_draw(
        ctx: Context<ExecuteDraw>,
        round_number: u64,
    ) -> Result<()> {
        execute_draw::handler(ctx, round_number)
    }

    /// Winner claims their share of the 88% prize pool.
    /// Verifies the entry's distance equals round.min_distance.
    pub fn claim_winnings(
        ctx: Context<ClaimWinnings>,
        round_number: u64,
        entry_index: u32,
    ) -> Result<()> {
        claim_winnings::handler(ctx, round_number, entry_index)
    }

    /// Coordinator cancels an empty round after cutoff (no entries submitted).
    pub fn cancel_empty(
        ctx: Context<CancelEmpty>,
        round_number: u64,
    ) -> Result<()> {
        cancel_empty::handler(ctx, round_number)
    }
}
