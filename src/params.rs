use std::sync::Arc;

use axum::{
	extract::{ws::Message, Query},
	http::StatusCode,
	response::IntoResponse,
	Extension,
};
use futures_util::SinkExt;
use serde::Deserialize;

use crate::state::State;

#[derive(Debug, Deserialize)]
pub struct RedirectParams {
	pub state: String,
	pub code: String,
}

pub async fn handler(
	Query(params): Query<RedirectParams>,
	Extension(state): Extension<Arc<State>>,
) -> impl IntoResponse {
	handle_redirect(state, params).await
}

async fn handle_redirect(
	state: Arc<State>,
	RedirectParams {
		state: state_value,
		code,
	}: RedirectParams,
) -> impl IntoResponse {
	match state.0.write().await.remove(state_value.as_str()) {
		None => {
			log::warn!("state parameter not sent via ws before redirect");
			return (StatusCode::BAD_REQUEST, "state parameter not registered");
		}
		Some(mut rx) => {
			log::info!("code {} successfully sent for state {}", code, state_value);
			let data = serde_json::json!({
				"code": code,
				"state": state_value
			});
			let _ = rx
				.send(Message::Text(serde_json::to_string(&data).unwrap()))
				.await;

			return (StatusCode::OK, "You may now close this tab");
		}
	}
}
