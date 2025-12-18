use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use warp::ws::Message;

#[derive(Clone)]
pub struct ConnectionManager {
    connections: Arc<DashMap<String, mpsc::UnboundedSender<Message>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
        }
    }

    pub fn add(&self, player_id: String, sender: mpsc::UnboundedSender<Message>) {
        self.connections.insert(player_id, sender);
    }

    pub fn remove(&self, player_id: &str) {
        self.connections.remove(player_id);
    }

    pub fn broadcast(&self, message: Message) -> usize {
        let mut success_count = 0;
        for entry in self.connections.iter() {
            if entry.value().send(message.clone()).is_ok() {
                success_count += 1;
            }
        }
        success_count
    }

    pub fn count(&self) -> usize {
        self.connections.len()
    }

    pub fn get_connected_players(&self) -> Vec<String> {
        self.connections
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}
