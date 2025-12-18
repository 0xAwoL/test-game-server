use crate::types::{AuthRequest, AuthResponse, Claims, JWT_EXPIRATION_HOURS, SessionInfo};
use chrono::{Duration, Utc};
use dashmap::DashMap;
use jsonwebtoken::{EncodingKey, Header, encode};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use warp::{Rejection, Reply, reject, reply};

pub struct SolanaVerifier {
    rpc_client: RpcClient,
    required_token_mint: Pubkey,
    debug_mode: bool,
}

impl SolanaVerifier {
    pub fn new(rpc_url: &str, token_mint: &str, debug_mode: bool) -> Result<Self, String> {
        let required_token_mint =
            Pubkey::from_str(token_mint).map_err(|e| format!("Invalid token mint: {}", e))?;

        Ok(Self {
            rpc_client: RpcClient::new(rpc_url.to_string()),
            required_token_mint,
            debug_mode,
        })
    }

    pub fn verify_signature(
        &self,
        wallet_address: &str,
        message: &str,
        signature: &str,
    ) -> Result<bool, String> {
        if self.debug_mode {
            log::debug!(
                "DEBUG MODE: Skipping signature verification for {}",
                wallet_address
            );
            return Ok(true);
        }

        let pubkey = Pubkey::from_str(wallet_address)
            .map_err(|e| format!("Invalid wallet address: {}", e))?;

        let sig =
            Signature::from_str(signature).map_err(|e| format!("Invalid signature: {}", e))?;

        let message_bytes = message.as_bytes();

        Ok(sig.verify(pubkey.as_ref(), message_bytes))
    }

    pub async fn verify_token_ownership(&self, wallet_address: &str) -> Result<bool, String> {
        if self.debug_mode {
            log::debug!(
                "DEBUG MODE: Skipping token ownership verification for {}",
                wallet_address
            );
            return Ok(true);
        }

        let wallet_pubkey = Pubkey::from_str(wallet_address)
            .map_err(|e| format!("Invalid wallet address: {}", e))?;

        let token_accounts = self
            .rpc_client
            .get_token_accounts_by_owner(
                &wallet_pubkey,
                solana_client::rpc_request::TokenAccountsFilter::Mint(self.required_token_mint),
            )
            .map_err(|e| format!("Failed to fetch token accounts: {}", e))?;

        for _account in token_accounts {
            return Ok(true);
        }

        Ok(false)
    }
}

pub async fn handle_auth(
    auth_req: AuthRequest,
    verifier: Arc<SolanaVerifier>,
    sessions: Arc<DashMap<String, SessionInfo>>,
    jwt_secret: String,
) -> Result<impl Reply, Rejection> {
    if !verifier
        .verify_signature(
            &auth_req.wallet_address,
            &auth_req.message,
            &auth_req.signature,
        )
        .map_err(|_| reject::reject())?
    {
        return Err(reject::reject());
    }

    let has_token = verifier
        .verify_token_ownership(&auth_req.wallet_address)
        .await
        .map_err(|_| reject::reject())?;

    if !has_token {
        return Err(reject::reject());
    }

    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(JWT_EXPIRATION_HOURS))
        .unwrap()
        .timestamp() as usize;

    let claims = Claims {
        wallet_address: auth_req.wallet_address.clone(),
        player_id: auth_req.wallet_address.clone(),
        nickname: auth_req.nickname.clone(),
        exp: expiration,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|_| reject::reject())?;

    sessions.insert(
        auth_req.wallet_address.clone(),
        SessionInfo {
            jwt_token: token.clone(),
            nickname: auth_req.nickname,
            created_at: Instant::now(),
        },
    );

    Ok(reply::json(&AuthResponse {
        jwt_token: token,
        player_id: claims.player_id,
        expires_in: (JWT_EXPIRATION_HOURS * 3600) as u64,
    }))
}
