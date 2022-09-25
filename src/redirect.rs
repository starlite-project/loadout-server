use std::sync::Arc;

use axum::{extract::Query, http::StatusCode, response::IntoResponse, Extension};
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
	state.received_users.write().await.insert(state_value, code);

	(
		StatusCode::OK,
		"You may now close this tab, the application may take up to 5 seconds to refresh",
	)
}
