//! Adaptor swap integration types for the Fuego DEX.
//!
//! Defines the 7-state adaptor swap protocol, P2P message types,
//! and RPC request/response structs for `swap_method: "adaptor"`.
//!
//! ## Protocol States
//!
//! ```text
//! KEYS_EXCHANGED → ESCROW_FUNDED → PRESIGS_READY → CTR_LOCKED
//!                                                      │
//!                                               SECRET_REVEALED
//!                                                      │
//!                                                 XFG_SPENT
//!                                                      │
//!                            ┌──────────────────────┬───┘
//!                            ▼                      ▼
//!                       COMPLETED              REFUNDED
//! ```

use core::fmt;
use serde::{Deserialize, Serialize};

/// The 7 states of the adaptor swap protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdaptorSwapState {
    /// Both parties have exchanged MuSig2-aggregated public keys.
    KeysExchanged = 0,
    /// XFG has been sent to the joint Musig address.
    EscrowFunded = 1,
    /// Both parties have exchanged Schnorr nonces and partial presignatures.
    PresigsReady = 2,
    /// Counterparty coin (CTR) is locked with adaptor signature.
    CtrLocked = 3,
    /// Alice broadcast the complete Schnorr sig on XFG chain; Bob extracts t.
    SecretRevealed = 4,
    /// Bob uses the extracted tweak t to spend XFG escrow.
    XfgSpent = 5,
    /// Cooperative refund on timeout — both sign refund tx.
    Refunded = 6,
    /// Both sides confirmed finality.
    Completed = 7,
}

impl AdaptorSwapState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, AdaptorSwapState::Completed | AdaptorSwapState::Refunded)
    }

    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(AdaptorSwapState::KeysExchanged),
            1 => Some(AdaptorSwapState::EscrowFunded),
            2 => Some(AdaptorSwapState::PresigsReady),
            3 => Some(AdaptorSwapState::CtrLocked),
            4 => Some(AdaptorSwapState::SecretRevealed),
            5 => Some(AdaptorSwapState::XfgSpent),
            6 => Some(AdaptorSwapState::Refunded),
            7 => Some(AdaptorSwapState::Completed),
            _ => None,
        }
    }
}

impl fmt::Display for AdaptorSwapState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdaptorSwapState::KeysExchanged => write!(f, "KEYS_EXCHANGED"),
            AdaptorSwapState::EscrowFunded => write!(f, "ESCROW_FUNDED"),
            AdaptorSwapState::PresigsReady => write!(f, "PRESIGS_READY"),
            AdaptorSwapState::CtrLocked => write!(f, "CTR_LOCKED"),
            AdaptorSwapState::SecretRevealed => write!(f, "SECRET_REVEALED"),
            AdaptorSwapState::XfgSpent => write!(f, "XFG_SPENT"),
            AdaptorSwapState::Refunded => write!(f, "REFUNDED"),
            AdaptorSwapState::Completed => write!(f, "COMPLETED"),
        }
    }
}

// ─── P2P message types ──────────────────────────────────────────────────────

/// A MuSig2 public key + key aggregation coefficient commitment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MusigKeyInfo {
    /// The participant's compressed secp256k1 public key (33 bytes hex).
    pub pubkey_hex: String,
    /// Proof of possession (ECDSA signature over the swap UUID).
    pub pop_hex: String,
}

/// Nonce commitment exchange.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NonceExchange {
    /// The participant's nonce commitment `R = k * G` (33 bytes hex).
    pub nonce_commitment_hex: String,
}

/// Adaptor presignature exchange.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresigExchange {
    /// Adaptor presig: `s' = k + e*sk + t` (32 bytes hex).
    pub s_prime_hex: String,
    /// The nonce R used for this presig (33 bytes hex).
    pub nonce_hex: String,
    /// The adaptor pubkey `T = t * G` (33 bytes hex).
    pub tweak_commitment_hex: String,
}

/// Key exchange phase message.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptorKeyExchangeMsg {
    pub swap_uuid: String,
    pub my_pubkey: MusigKeyInfo,
}

/// Nonce exchange phase message.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptorNonceMsg {
    pub swap_uuid: String,
    pub my_nonce: NonceExchange,
}

/// Presignature exchange phase message.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptorPresigMsg {
    pub swap_uuid: String,
    pub my_presig: PresigExchange,
}

/// Escrow funded notification (broadcast when XFG tx is confirmed).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptorEscrowFundedMsg {
    pub swap_uuid: String,
    /// XFG escrow transaction ID.
    pub escrow_txid: String,
    /// Block height where escrow was confirmed.
    pub escrow_height: u64,
}

/// CTR locked notification (broadcast when counterparty coin is locked).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptorCtrLockedMsg {
    pub swap_uuid: String,
    /// CTR lock transaction ID.
    pub ctr_txid: String,
    /// Block height.
    pub ctr_height: u64,
}

/// Secret revealed notification (on-chain sig detected by taker).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptorSecretRevealedMsg {
    pub swap_uuid: String,
    /// The extracted tweak t (32 bytes hex).
    pub tweak_hex: String,
}

// ─── RPC types ──────────────────────────────────────────────────────────────

/// The `swap_method` field value for adaptor signature swaps.
pub const ADAPTOR_SWAP_METHOD: &str = "adaptor";

/// Setprice / buy / sell order with `swap_method: "adaptor"`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptorOrderRequest {
    pub base: String,
    pub rel: String,
    #[serde(rename = "swap_method")]
    pub swap_method: String, // "adaptor"
    pub price: String,
    pub volume: String,
    #[serde(default)]
    pub timeout: Option<u64>,
}

/// Order returned by orderbook with swap_method field.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptorOrderEntry {
    pub coin: String,
    pub address: String,
    pub price: String,
    #[serde(rename = "numutxos")]
    pub num_utxos: u64,
    pub avevolume: String,
    pub maxvolume: String,
    pub depth: String,
    pub pubkey: String,
    pub age: u64,
    pub zcredits: u64,
    #[serde(default, rename = "swap_method")]
    pub swap_method: Option<String>,
}

/// Status response for an adaptor swap.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptorSwapStatus {
    pub uuid: String,
    pub state: AdaptorSwapState,
    pub base_coin: String,
    pub rel_coin: String,
    pub my_amount: String,
    pub other_amount: String,
    pub started_at: u64,
    pub escrow_txid: Option<String>,
    pub ctr_txid: Option<String>,
    pub aggregated_pubkey: Option<String>,
    pub tweak_commitment: Option<String>,
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_serialization() {
        let state = AdaptorSwapState::KeysExchanged;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"KEYS_EXCHANGED\"");
        let back: AdaptorSwapState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, AdaptorSwapState::KeysExchanged);
    }

    #[test]
    fn test_state_roundtrip() {
        for v in 0..=7u8 {
            let state = AdaptorSwapState::from_u8(v).unwrap();
            assert_eq!(state.as_u8(), v);
        }
    }

    #[test]
    fn test_terminal_states() {
        assert!(AdaptorSwapState::Completed.is_terminal());
        assert!(AdaptorSwapState::Refunded.is_terminal());
        assert!(!AdaptorSwapState::KeysExchanged.is_terminal());
    }

    #[test]
    fn test_presig_msg_serialization() {
        let msg = PresigExchange {
            s_prime_hex: "aabbccdd".into(),
            nonce_hex: "11223344".into(),
            tweak_commitment_hex: "55667788".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: PresigExchange = serde_json::from_str(&json).unwrap();
        assert_eq!(back.s_prime_hex, "aabbccdd");
    }
}
