use std::collections::HashMap;

use tokio::sync::RwLock;

#[derive(Debug, Default)]
pub struct State {
	pub received_users: RwLock<HashMap<String, String>>,
	pub api_keys: Vec<String>,
}

impl State {
	pub fn new(keys: Vec<String>) -> Self {
		Self {
			api_keys: keys,
			..Self::default()
		}
	}
}
