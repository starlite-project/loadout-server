use std::sync::Arc;

use axum::{
	body::Body,
	extract::Query,
	http::{Request, StatusCode},
	response::{IntoResponse, Response},
	Extension,
};
use serde::Deserialize;
use serde_json::Value;

use crate::state::State;

#[derive(Debug, Deserialize)]
pub struct RetrievalParams {
	state: String,
}

pub async fn handler(
	Extension(state): Extension<Arc<State>>,
	Query(RetrievalParams { state: state_value }): Query<RetrievalParams>,
	request: Request<Body>,
) -> Response {
	match request.headers().get("x-api-key") {
		None => return (StatusCode::BAD_REQUEST, "No X-Api-Key present").into_response(),
		Some(key) => {
			if !state
				.api_keys
				.contains(&key.to_str().unwrap_or_default().to_owned())
			{
				return (StatusCode::BAD_REQUEST, "X-Api-Key doesn't match").into_response();
			}
		}
	};

	let value = state
		.received_users
		.write()
		.await
		.remove(&state_value)
		.map(Value::String)
		.unwrap_or(Value::Null);

	match serde_json::to_string(&value) {
		Ok(v) => v.into_response(),
		Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)).into_response(),
	}
}
