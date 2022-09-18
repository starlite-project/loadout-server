use std::sync::Arc;

use axum::{extract::WebSocketUpgrade, response::Response, Extension};

use crate::state::State;

pub async fn handler(ws: WebSocketUpgrade, Extension(state): Extension<Arc<State>>) -> Response {
	ws.on_upgrade(|socket| state.handle_socket(socket))
}