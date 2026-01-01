# Pep Game Server

Essentially my playground for learning how to build real-time multiplayer systems using Rust's async ecosystem + the actor pattern.

### What does it do?

It gives you a foundation for a token-gated multiplayer game server that can:

- **Token-gate access** — only players holding the right NFT or SPL token can join  
- Handle **real-time movement & game logic** with very low latency (blockchain stays far away from the hot path)  

### Diagram

### How it actually works under the hood
- **Authentication**  
  Uses `solana-client` to verify wallet signatures + check token balances for a specific mint

- **Concurrency & State**  
  Everything is built around the beautiful little [`tiny-tokio-actor`](https://crates.io/crates/tiny-tokio-actor) library  
  → Each player, zone, or important entity lives as its own actor

- **Networking**  
  Simple & fast WebSocket server powered by `warp`  
  Bidirectional, persistent connections for sending/receiving game events

- **Session handling**  
  Secure sessions with JWT validation + graceful connection tracking

### Quick Start

```bash
git clone <your-repo-url-here>
cd pep-game-server
cp .env.example .env # edit .env
cargo check
cargo run
