use std::collections::HashMap;

use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::SplitSink;
use tokio::sync::RwLock;

#[derive(Debug, Default)]
pub struct State(pub RwLock<HashMap<String, SplitSink<WebSocket, Message>>>);
