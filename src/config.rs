use std::env;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub debug_mode: bool,
    pub rpc_url: String,
    pub token_mint: String,
    pub jwt_secret: String,
    pub tickrate_ms: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 9000,
            debug_mode: false,
            rpc_url: "https://api.devnet.solana.com".to_string(),
            token_mint: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
            jwt_secret: "your-secret-key-change-in-production".to_string(),
            tickrate_ms: 4,
        }
    }
}

impl ServerConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(url) = env::var("SOLANA_RPC_URL") {
            config.rpc_url = url;
        }

        if let Ok(mint) = env::var("TOKEN_MINT_ADDRESS") {
            config.token_mint = mint;
        }

        if let Ok(secret) = env::var("JWT_SECRET") {
            config.jwt_secret = secret;
        }

        if let Ok(port) = env::var("PORT") {
            if let Ok(p) = port.parse::<u16>() {
                config.port = p;
            }
        }

        if let Ok(debug) = env::var("DEBUG_MODE") {
            config.debug_mode = debug.parse::<bool>().unwrap_or(false);
        }

        if let Ok(tickrate) = env::var("TICKRATE_MS") {
            if let Ok(t) = tickrate.parse::<u64>() {
                config.tickrate_ms = t;
            }
        }

        config
    }
}
