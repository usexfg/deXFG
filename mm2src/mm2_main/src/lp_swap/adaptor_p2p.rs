//! P2P message types for the adaptor signature swap protocol.
//!
//! These messages are exchanged over gossipsub on topics `"adswap/<uuid>"`.
//! They follow the same pattern as V2 swap messages (`swap_v2.proto`) but
//! are defined as native Rust structs for simpler integration.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Topic prefix for adaptor swap P2P messages.
pub const ADAPTOR_SWAP_PREFIX: &str = "adswap";

/// Build a P2P topic for an adaptor swap.
pub fn adaptor_swap_topic(uuid: &Uuid) -> String {
    format!("{}/{}", ADAPTOR_SWAP_PREFIX, uuid)
}

// ─── Key exchange phase ─────────────────────────────────────────────────────

/// Maker broadcasts their Musig2 public key and proof-of-possession.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptorKeyExchange {
    /// Compressed secp256k1 public key (33 bytes, hex).
    pub pubkey_hex: String,
    /// ECDSA signature over the swap UUID as proof of possession.
    pub pop_hex: String,
}

// ─── Nonce exchange phase ────────────────────────────────────────────────────

/// Both parties exchange Schnorr nonce commitments.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptorNonceExchange {
    /// Compressed secp256k1 nonce commitment `R = k * G` (33 bytes, hex).
    pub nonce_commitment_hex: String,
}

// ─── Presignature exchange phase ─────────────────────────────────────────────

/// Maker sends their adaptor presignature and tweak commitment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptorPresigExchange {
    /// Adaptor presig scalar `s'` (32 bytes, hex).
    pub s_prime_hex: String,
    /// The nonce `R` used for this presig (33 bytes, hex).
    pub nonce_hex: String,
    /// The tweak commitment `T = t * G` (33 bytes, hex).
    pub tweak_commitment_hex: String,
}

// ─── Escrow and lock notifications ───────────────────────────────────────────

/// Broadcast when XFG escrow is confirmed on-chain.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptorEscrowFunded {
    /// XFG escrow transaction ID.
    pub escrow_txid: String,
    /// Block height at which escrow was confirmed.
    pub escrow_height: u64,
}

/// Broadcast when the counterparty coin (CTR) is locked.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptorCtrLocked {
    /// CTR lock transaction ID.
    pub ctr_txid: String,
    /// Block height.
    pub ctr_height: u64,
}

// ─── Secret extraction notification ──────────────────────────────────────────

/// Taker sends the extracted tweak to the maker (or broadcasts on-chain).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptorSecretRevealed {
    /// The extracted tweak `t` (32 bytes, hex).
    pub tweak_hex: String,
}

// ─── Unified message enum ────────────────────────────────────────────────────

/// All adaptor swap P2P message variants.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum AdaptorSwapMessage {
    #[serde(rename = "key_exchange")]
    KeyExchange(AdaptorKeyExchange),
    #[serde(rename = "nonce_exchange")]
    NonceExchange(AdaptorNonceExchange),
    #[serde(rename = "presig_exchange")]
    PresigExchange(AdaptorPresigExchange),
    #[serde(rename = "escrow_funded")]
    EscrowFunded(AdaptorEscrowFunded),
    #[serde(rename = "ctr_locked")]
    CtrLocked(AdaptorCtrLocked),
    #[serde(rename = "secret_revealed")]
    SecretRevealed(AdaptorSecretRevealed),
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_format() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let topic = adaptor_swap_topic(&uuid);
        assert_eq!(topic, "adswap/550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_key_exchange_serialization() {
        let msg = AdaptorSwapMessage::KeyExchange(AdaptorKeyExchange {
            pubkey_hex: "02aabb".into(),
            pop_hex: "30440220deadbeef".into(),
        });
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("key_exchange"));
        let back: AdaptorSwapMessage = serde_json::from_str(&json).unwrap();
        match back {
            AdaptorSwapMessage::KeyExchange(ke) => {
                assert_eq!(ke.pubkey_hex, "02aabb");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_presig_exchange_serialization() {
        let msg = AdaptorSwapMessage::PresigExchange(AdaptorPresigExchange {
            s_prime_hex: "aabbccdd".into(),
            nonce_hex: "02eeff".into(),
            tweak_commitment_hex: "031122".into(),
        });
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("presig_exchange"));
        let back: AdaptorSwapMessage = serde_json::from_str(&json).unwrap();
        match back {
            AdaptorSwapMessage::PresigExchange(pe) => {
                assert_eq!(pe.s_prime_hex, "aabbccdd");
                assert_eq!(pe.tweak_commitment_hex, "031122");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_all_variants_roundtrip() {
        let messages = vec![
            AdaptorSwapMessage::KeyExchange(AdaptorKeyExchange {
                pubkey_hex: "pk".into(),
                pop_hex: "pop".into(),
            }),
            AdaptorSwapMessage::NonceExchange(AdaptorNonceExchange {
                nonce_commitment_hex: "nc".into(),
            }),
            AdaptorSwapMessage::EscrowFunded(AdaptorEscrowFunded {
                escrow_txid: "txid".into(),
                escrow_height: 100,
            }),
            AdaptorSwapMessage::CtrLocked(AdaptorCtrLocked {
                ctr_txid: "ctx".into(),
                ctr_height: 200,
            }),
            AdaptorSwapMessage::SecretRevealed(AdaptorSecretRevealed {
                tweak_hex: "tweak".into(),
            }),
        ];

        for msg in messages {
            let json = serde_json::to_string(&msg).unwrap();
            let back: AdaptorSwapMessage = serde_json::from_str(&json).unwrap();
            let re_json = serde_json::to_string(&back).unwrap();
            assert_eq!(json, re_json);
        }
    }
}
