//! Fuegod native RPC client.
//!
//! Thin HTTP client wrapping fuegod's REST and JSON-RPC endpoints.
//! Used by the adaptor state machine for:
//! - Chain operations: get height, broadcast tx, mine blocks (testnet)
//! - Adaptor swap lifecycle: initiate, accept, process, refund, status
//! - Swap orderbook: submit offers, list offers, cancel offers, request swap

use serde::{Deserialize, Serialize};

/// Fuegod daemon RPC client.
pub struct FuegodClient {
    base_url: String,
    client: reqwest::Client,
}

// ─── Chain data types ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct FuegodInfo {
    pub height: u64,
    pub difficulty: u64,
    pub tx_count: u64,
    pub tx_pool_size: u64,
    pub incoming_connections_count: u64,
    pub outgoing_connections_count: u64,
    pub last_block_timestamp: u64,
    pub last_block_reward: u64,
    pub top_block_hash: String,
    pub status: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct BlockCount {
    pub count: u64,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct SendRawTxResponse {
    pub status: String,
}

// ─── Swap offer types ───────────────────────────────────────────────────────

/// A single swap offer entry from /getswapoffers
#[derive(Debug, Deserialize)]
pub struct SwapOfferEntry {
    pub offer_id: String,
    pub maker_pub_key: String,
    pub xfg_amount: u64,
    pub ctr_amount: u64,
    pub pair: u8,
    pub rate_num: u64,
    pub signature: String,
    pub ttl_blocks: u32,
    pub posted_height: u32,
    pub timestamp: u64,
    #[serde(default)]
    pub is_soft_order: bool,
}

#[derive(Debug, Deserialize)]
pub struct GetSwapOffersResponse {
    pub offers: Vec<SwapOfferEntry>,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct SubmitSwapOfferRequest {
    pub offer_id: String,
    pub xfg_amount: u64,
    pub rate_num: u64,
    pub pair: u8,
    pub maker_pub_key: String,
    pub signature: String,
    pub ttl_blocks: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_soft_order: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CancelSwapOfferRequest {
    pub offer_id: String,
    pub maker_pub_key: String,
    pub signature: String,
}

#[derive(Debug, Serialize)]
pub struct RequestSwapRequest {
    pub offer_id: String,
    pub amount: u64,
    pub taker_pub_key: String,
    pub proof_of_funds: String,
}

#[derive(Debug, Serialize)]
pub struct InitiateSwapRequest {
    pub pair: String,
    pub xfg_amount: u64,
    pub ctr_amount: u64,
    pub ctr_address: String,
    pub peer_endpoint: String,
    pub peer_pub_key: String,
}

#[derive(Debug, Deserialize)]
pub struct InitiateSwapResponse {
    pub swap_id: String,
    #[serde(default)]
    pub our_pub_key: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct SwapIdRequest {
    pub swap_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ProcessSwapResponse {
    pub advanced: bool,
    #[serde(default)]
    pub new_state: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct SwapStatusResponse {
    pub swap_id: String,
    pub state: String,
    pub pair: String,
    pub role: String,
    pub xfg_amount: u64,
    pub ctr_address: String,
    pub peer_endpoint: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub is_terminal: bool,
    pub found: bool,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct SwapSummary {
    pub swap_id: String,
    pub state: String,
    pub pair: String,
    pub role: String,
    pub xfg_amount: u64,
    pub created_at: u64,
    pub updated_at: u64,
    pub is_terminal: bool,
}

#[derive(Debug, Deserialize)]
pub struct ListSwapsResponse {
    pub swaps: Vec<SwapSummary>,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct OfferPairRequest {
    pub pair: u8,
}

// ─── Internal types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StatusResponse {
    pub status: String,
}

#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: String,
    method: String,
    params: serde_json::Value,
}

/// Helper: POST to a REST endpoint, parse JSON response
async fn post_json<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    body: &impl Serialize,
) -> Result<T, String> {
    let resp = client
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;
    resp.json::<T>()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))
}

// ─── FuegodClient implementation ────────────────────────────────────────────

impl FuegodClient {
    pub fn new(base_url: &str) -> Self {
        FuegodClient {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    // ── Chain queries ────────────────────────────────────────────────────────

    pub async fn get_info(&self) -> Result<FuegodInfo, String> {
        let url = format!("{}/getinfo", self.base_url);
        self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?
            .json::<FuegodInfo>()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))
    }

    pub async fn get_block_count(&self) -> Result<u64, String> {
        let resp = self
            .json_rpc::<BlockCount>("getblockcount", serde_json::json!({}))
            .await?;
        Ok(resp.count)
    }

    pub async fn get_block_hash(&self, height: u64) -> Result<String, String> {
        self.json_rpc::<String>("on_getblockhash", serde_json::json!([height]))
            .await
    }

    pub async fn get_height(&self) -> Result<u64, String> {
        self.get_block_count().await
    }

    // ── Transaction ──────────────────────────────────────────────────────────

    pub async fn send_raw_tx(&self, tx_hex: &str) -> Result<SendRawTxResponse, String> {
        let url = format!("{}/sendrawtransaction", self.base_url);
        post_json(&self.client, &url, &serde_json::json!({ "tx_as_hex": tx_hex })).await
    }

    // ── Mining (testnet) ─────────────────────────────────────────────────────

    pub async fn start_mining(&self, threads: u32, address: &str) -> Result<String, String> {
        let url = format!("{}/start_mining", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({ "miner_address": address, "threads_count": threads }))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;
        resp.text().await.map_err(|e| format!("Read error: {}", e))
    }

    pub async fn stop_mining(&self) -> Result<String, String> {
        let url = format!("{}/stop_mining", self.base_url);
        let resp = self
            .client
            .post(&url)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;
        resp.text().await.map_err(|e| format!("Read error: {}", e))
    }

    pub async fn mine_one_block(&self, address: &str) -> Result<u64, String> {
        let before = self.get_height().await?;
        self.start_mining(1, address).await?;
        for _ in 0..120 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let after = self.get_height().await?;
            if after > before {
                self.stop_mining().await?;
                return Ok(after);
            }
        }
        self.stop_mining().await?;
        Err("Mining timeout: no block found after 120 seconds".to_string())
    }

    pub async fn mine_blocks(&self, count: u64, address: &str) -> Result<u64, String> {
        let mut height = self.get_height().await?;
        for i in 0..count {
            height = self.mine_one_block(address).await?;
            log::info!("Mined block {} → height {}", i + 1, height);
        }
        Ok(height)
    }

    // ── Swap orderbook ───────────────────────────────────────────────────────

    /// List swap offers for a given pair (0=SOL, 1=ETH, 2=XMR, 3=BCH).
    pub async fn get_swap_offers(&self, pair: u8) -> Result<GetSwapOffersResponse, String> {
        let url = format!("{}/getswapoffers", self.base_url);
        post_json(&self.client, &url, &OfferPairRequest { pair }).await
    }

    /// Submit a swap offer to the orderbook.
    pub async fn submit_swap_offer(&self, req: &SubmitSwapOfferRequest) -> Result<StatusResponse, String> {
        let url = format!("{}/submitswap", self.base_url);
        post_json(&self.client, &url, req).await
    }

    /// Cancel an existing swap offer.
    pub async fn cancel_swap_offer(&self, req: &CancelSwapOfferRequest) -> Result<StatusResponse, String> {
        let url = format!("{}/cancelswap", self.base_url);
        post_json(&self.client, &url, req).await
    }

    /// Request a swap from an existing offer.
    pub async fn request_swap(&self, req: &RequestSwapRequest) -> Result<StatusResponse, String> {
        let url = format!("{}/requestswap", self.base_url);
        post_json(&self.client, &url, req).await
    }

    // ── Swap lifecycle ───────────────────────────────────────────────────────

    /// Initiate a new adaptor swap. Returns the swap_id on success.
    /// peer_endpoint: the peer's fuegod address for direct daemon↔daemon protocol.
    /// peer_pub_key: the peer's Ed25519 Musig2 public key (hex).
    pub async fn initiate_swap(&self, req: &InitiateSwapRequest) -> Result<InitiateSwapResponse, String> {
        let url = format!("{}/initiate", self.base_url);
        post_json(&self.client, &url, req).await
    }

    /// Accept a pending swap (the counterparty side).
    pub async fn accept_swap(&self, swap_id: &str) -> Result<StatusResponse, String> {
        let url = format!("{}/accept", self.base_url);
        post_json(&self.client, &url, &SwapIdRequest { swap_id: swap_id.into() }).await
    }

    /// Advance the swap state machine. Returns the new state if it advanced.
    pub async fn process_swap(&self, swap_id: &str) -> Result<ProcessSwapResponse, String> {
        let url = format!("{}/processswap", self.base_url);
        post_json(&self.client, &url, &SwapIdRequest { swap_id: swap_id.into() }).await
    }

    /// Refund a timed-out swap.
    pub async fn refund_swap(&self, swap_id: &str) -> Result<StatusResponse, String> {
        let url = format!("{}/refundswap", self.base_url);
        post_json(&self.client, &url, &SwapIdRequest { swap_id: swap_id.into() }).await
    }

    /// Get detailed status of a swap.
    pub async fn get_swap_status(&self, swap_id: &str) -> Result<SwapStatusResponse, String> {
        let url = format!("{}/getswapstatus", self.base_url);
        post_json(&self.client, &url, &SwapIdRequest { swap_id: swap_id.into() }).await
    }

    /// List all swaps known to the daemon.
    pub async fn list_swaps(&self) -> Result<ListSwapsResponse, String> {
        let url = format!("{}/listswaps", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;
        resp.json::<ListSwapsResponse>()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))
    }

    // ── Internal ─────────────────────────────────────────────────────────────

    async fn json_rpc<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T, String> {
        let url = format!("{}/json_rpc", self.base_url);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: "1".to_string(),
            method: method.to_string(),
            params,
        };
        let resp = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        if let Some(error) = body.get("error") {
            return Err(format!("RPC error: {}", error));
        }

        let result = serde_json::from_value::<T>(body["result"].clone())
            .map_err(|e| format!("Result parse error: {} from {}", e, body["result"]))?;
        Ok(result)
    }
}

// ─── Unit tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = FuegodClient::new("http://127.0.0.1:28280");
        assert_eq!(client.base_url, "http://127.0.0.1:28280");
    }

    #[test]
    fn test_json_rpc_request_serialization() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: "1".into(),
            method: "getblockcount".into(),
            params: serde_json::json!({}),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("getblockcount"));
    }

    #[test]
    fn test_initiate_swap_request_serialization() {
        let req = InitiateSwapRequest {
            pair: "SOL".into(),
            xfg_amount: 100_000_000_000,
            ctr_amount: 1_000_000_000,
            ctr_address: "address".into(),
            peer_endpoint: "127.0.0.1:28280".into(),
            peer_pub_key: "deadbeef".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("SOL"));
        assert!(json.contains("xfg_amount"));
    }

    #[test]
    fn test_swap_offer_request_serialization() {
        let req = SubmitSwapOfferRequest {
            offer_id: "offer1".into(),
            xfg_amount: 100_000_000_000,
            rate_num: 1_000_000,
            pair: 0,
            maker_pub_key: "abcd".into(),
            signature: "sig".into(),
            ttl_blocks: 60,
            is_soft_order: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("offer1"));
    }
}

// ─── Integration tests (live fuegod) ────────────────────────────────────────

#[cfg(test)]
mod integration {
    use super::*;

    const FUEGOD_URL: &str = "http://127.0.0.1:28280";

    #[tokio::test]
    async fn test_get_info() {
        let client = FuegodClient::new(FUEGOD_URL);
        let info = client.get_info().await.expect("get_info failed");
        println!("Height: {}, Version: {}", info.height, info.version);
        assert!(info.height > 0);
        assert_eq!(info.status, "OK");
    }

    #[tokio::test]
    async fn test_get_block_count() {
        let client = FuegodClient::new(FUEGOD_URL);
        let count = client.get_block_count().await.expect("get_block_count failed");
        println!("Block count: {}", count);
        assert!(count > 0);
    }

    #[tokio::test]
    async fn test_get_block_hash() {
        let client = FuegodClient::new(FUEGOD_URL);
        let hash = client.get_block_hash(0).await.expect("get_block_hash failed");
        println!("Genesis hash: {}", hash);
        assert_eq!(hash.len(), 64);
    }

    #[tokio::test]
    async fn test_list_swaps() {
        let client = FuegodClient::new(FUEGOD_URL);
        let resp = client.list_swaps().await.expect("list_swaps failed");
        println!("Swaps: {} (status: {})", resp.swaps.len(), resp.status);
        assert_eq!(resp.status, "OK");
    }

    #[tokio::test]
    async fn test_get_swap_offers() {
        let client = FuegodClient::new(FUEGOD_URL);
        let resp = client.get_swap_offers(0).await.expect("get_swap_offers failed");
        println!("Offers for pair 0: {} (status: {})", resp.offers.len(), resp.status);
        assert_eq!(resp.status, "OK");
    }
}
