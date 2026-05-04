use anchor_lang::prelude::*;

/// Clawtery custom error codes.
/// All errors return descriptive messages for debugging.
#[error_code]
pub enum ClawteryError {
    #[msg("Unauthorized: signer does not have permission for this action")]
    Unauthorized,
    
    #[msg("Round is not open for entries")]
    RoundNotOpen,
    
    #[msg("Submission window has closed (cutoff time reached)")]
    SubmissionClosed,
    
    #[msg("Round is not in committed state")]
    RoundNotCommitted,
    
    #[msg("Round has already been drawn")]
    RoundAlreadyDrawn,
    
    #[msg("Committed draw slot has not been reached yet")]
    SlotNotReached,
    
    #[msg("SlotHashes sysvar is not available")]
    SlotHashesUnavailable,
    
    #[msg("Target slot not found in SlotHashes sysvar")]
    SlotNotFound,
    
    #[msg("No entries submitted for this round")]
    NoEntries,
    
    #[msg("Invalid prediction value")]
    InvalidPrediction,
    
    #[msg("Entry cost does not match required amount")]
    EntryCostMismatch,
    
    #[msg("Draw time has not been reached")]
    DrawTimeNotReached,
    
    #[msg("Too early to commit draw (before cutoff time)")]
    CommitTooEarly,
    
    #[msg("Round number must be sequential (current_round + 1)")]
    InvalidRoundNumber,
    
    #[msg("No winners found in this round")]
    NoWinners,
    
    #[msg("Invalid entry account data")]
    InvalidEntry,
    
    #[msg("Priority fee exceeds maximum allowed")]
    PriorityFeeTooHigh,
}
