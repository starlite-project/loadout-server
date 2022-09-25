use std::sync::Arc;

use axum::{
	body::Body,
	http::{Request, StatusCode},
	response::IntoResponse,
	Extension,
};

use crate::state::State;

pub async fn handler(
	Extension(state): Extension<Arc<State>>,
	request: Request<Body>,
) -> impl IntoResponse {
	let headers = request.headers();

	// let api_key = headers.get("x-api-key");
	let api_key = if let Some(key) = headers.get("x-api-key") {
		key.clone()
	} else {
		return (StatusCode::BAD_REQUEST, "No X-Api-Key present");
	};

	(StatusCode::OK, "")
}
