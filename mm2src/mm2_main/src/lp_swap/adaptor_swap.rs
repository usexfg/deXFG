//! Adaptor signature swap for Fuego DEX.
//!
//! This module provides the entry points for adaptor signature swaps (XFG pairs).
//! The full 7-state machine will be implemented incrementally as the adaptor
//! crypto module (`fuego_adaptor`) matures.

use coins::MmCoinEnum;
use common::HttpStatusCode;
use derive_more::Display;
use http::StatusCode;
use mm2_err_handle::prelude::*;
use ser_error_derive::SerializeErrorType;
use serde::Serialize;
use uuid::Uuid;

/// Error type for adaptor swap operations.
#[derive(Clone, Debug, Display, Serialize, SerializeErrorType)]
#[serde(tag = "error_type", content = "error_data")]
pub enum AdaptorSwapError {
    #[display(fmt = "Adaptor swap not yet implemented: {}", _0)]
    NotImplemented(String),
    #[display(fmt = "Internal error: {}", _0)]
    Internal(String),
}

impl HttpStatusCode for AdaptorSwapError {
    fn status_code(&self) -> StatusCode {
        match self {
            AdaptorSwapError::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
            AdaptorSwapError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Whether the given coin/ticker supports adaptor signature swaps.
pub fn is_adaptor_capable(ticker: &str) -> bool {
    ticker == "XFG" || ticker == "HEAT"
}

/// Check if both sides of a swap pair are compatible with adaptor swaps.
/// At least one side must be XFG; both sides must support their respective
/// half of the protocol (XFG side does adaptor sig, counterparty uses HTLC).
pub fn is_adaptor_swap_pair(maker_coin: &MmCoinEnum, taker_coin: &MmCoinEnum) -> bool {
    let m = maker_coin.ticker();
    let t = taker_coin.ticker();
    is_adaptor_capable(m) || is_adaptor_capable(t)
}

/// Entry point: start a maker-side (order creator) adaptor swap.
///
/// Called from `lp_connect_start_bob` when the swap pair involves XFG.
pub async fn start_adaptor_maker_swap(
    _maker_coin: &MmCoinEnum,
    _taker_coin: &MmCoinEnum,
    _uuid: Uuid,
) -> Result<(), MmError<AdaptorSwapError>> {
    Err(MmError::new(AdaptorSwapError::NotImplemented(
        "Adaptor maker swap state machine not yet implemented".into(),
    )))
}

/// Entry point: start a taker-side adaptor swap.
pub async fn start_adaptor_taker_swap(
    _maker_coin: &MmCoinEnum,
    _taker_coin: &MmCoinEnum,
    _uuid: Uuid,
) -> Result<(), MmError<AdaptorSwapError>> {
    Err(MmError::new(AdaptorSwapError::NotImplemented(
        "Adaptor taker swap state machine not yet implemented".into(),
    )))
}

/// The 7 states of the adaptor swap protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AdaptorState {
    KeysExchanged = 0,
    EscrowFunded = 1,
    PresigsReady = 2,
    CtrLocked = 3,
    SecretRevealed = 4,
    XfgSpent = 5,
    Refunded = 6,
    Completed = 7,
}

impl AdaptorState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, AdaptorState::Completed | AdaptorState::Refunded)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_adaptor_capable() {
        assert!(is_adaptor_capable("XFG"));
        assert!(is_adaptor_capable("HEAT"));
        assert!(!is_adaptor_capable("KMD"));
        assert!(!is_adaptor_capable("BTC"));
    }

    #[test]
    fn test_terminal_states() {
        assert!(AdaptorState::Completed.is_terminal());
        assert!(AdaptorState::Refunded.is_terminal());
        assert!(!AdaptorState::KeysExchanged.is_terminal());
    }

    #[test]
    fn test_state_values() {
        assert_eq!(AdaptorState::KeysExchanged as u8, 0);
        assert_eq!(AdaptorState::Completed as u8, 7);
    }
}
