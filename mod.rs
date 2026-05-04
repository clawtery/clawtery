//! Clawtery instruction handlers.
//!
//! Each module implements one program instruction:
//! - `initialize` — Set up global config
//! - `start_round` — Create a new round
//! - `enter` — Submit a prediction
//! - `commit_draw` — Coordinator commits draw parameters
//! - `execute_draw` — Execute draw, compute winner, distribute shares
//! - `claim_winnings` — Winner collects prize
//! - `cancel_empty` — Cancel round with no entries

pub mod initialize;
pub mod start_round;
pub mod enter;
pub mod commit_draw;
pub mod execute_draw;
pub mod claim_winnings;
pub mod cancel_empty;

// Re-export all handler functions for convenience
pub use initialize::*;
pub use start_round::*;
pub use enter::*;
pub use commit_draw::*;
pub use execute_draw::*;
pub use claim_winnings::*;
pub use cancel_empty::*;
