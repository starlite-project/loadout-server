use std::sync::Arc;

use axum::{extract::Query, response::IntoResponse, Extension};
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
	state.handle_redirect(params).await
}
