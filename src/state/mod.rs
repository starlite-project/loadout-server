use std::{collections::HashMap, env, sync::Arc, time::Duration};

use axum::{
	extract::ws::{Message, WebSocket},
	http::StatusCode,
	response::IntoResponse,
};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use serde::Deserialize;
use tokio::{
	sync::RwLock,
	time::{interval, timeout},
};

use self::close::CloseCode;
use crate::params::RedirectParams;

mod close;

#[derive(Debug, Default)]
pub struct State(RwLock<HashMap<String, SplitSink<WebSocket, Message>>>);

impl State {
	pub async fn handle_socket(self: Arc<Self>, mut ws: WebSocket) {
		let api_keys = match env::var("API_KEYS") {
			Ok(v) => v.split(",").map(ToOwned::to_owned).collect::<Vec<_>>(),
			Err(_) => unreachable!(),
		};

		let first_message =
			if let Ok(Some(Ok(msg))) = timeout(Duration::from_secs(15), ws.next()).await {
				msg
			} else {
				log::error!("didn't receive auth message within 15 seconds, closing connection");
				let _ = ws
					.send(
						CloseCode::TryAgainLater
							.into_close_message("timed out waiting for auth message"),
					)
					.await;

				return;
			};

		if !matches!(first_message, Message::Text(_)) {
			log::error!("auth message was not sent as text");
			let _ = ws
				.send(
					CloseCode::UnsupportedData
						.into_close_message("expected text-based auth message"),
				)
				.await;
			return;
		}

		let message_data =
			match serde_json::from_slice::<MessageData>(first_message.into_data().as_ref()) {
				Err(e) => {
					log::error!("message data was not valid JSON: {}", e);
					let _ = ws
						.send(
							CloseCode::UnsupportedData
								.into_close_message("expected valid JSON data"),
						)
						.await;

					return;
				}
				Ok(d) => d,
			};

		log::info!("got auth message from {}", message_data.state);

		if !api_keys.contains(&message_data.api_key) {
			log::error!("api key didn't match set keys from {}", message_data.state);
			let _ = ws
				.send(CloseCode::Unauthorized.into_close_message("api_key was invalid"))
				.await;
			return;
		}

		let (user_ws_tx, mut user_ws_rx) = ws.split();

		self.0
			.write()
			.await
			.insert(message_data.state.clone(), user_ws_tx);

		let mut ping_interval = interval(Duration::from_secs(5));

		// the first ping is immediate, so get it out of the way
		ping_interval.tick().await;

		tokio::spawn(async move {
			loop {
				tokio::select! {
					Some(Ok(msg)) = user_ws_rx.next() => {
						if matches!(msg, Message::Close(_)) {
							log::info!("was sent close code, closing connection {}", message_data.state);
							self.0.write().await.remove(message_data.state.as_str());
							return;
						}
					}
					_ = ping_interval.tick() => {
						log::debug!("sending ping to connection {}", message_data.state);
						if let Some(tx) = self.0.write().await.get_mut(message_data.state.as_str()) {
							let ping = Message::Ping(vec![]);
							let _ = tx.send(ping).await;
						}
					}
				}
			}
		})
		.await
		.unwrap();
	}

	pub async fn handle_redirect(
		self: Arc<Self>,
		RedirectParams { state, code }: RedirectParams,
	) -> impl IntoResponse {
		match self.0.write().await.remove(state.as_str()) {
			None => {
				log::warn!("state parameter not sent via ws before redirect");
				return (StatusCode::BAD_REQUEST, "state parameter not registered");
			}
			Some(mut rx) => {
				log::info!("code {} successfully sent for state {}", code, state);
				// let _ = rx.send(Message::Text(serde_json::json!(
				//     "code": code,
				//     "state": state
				// )).await;
				let data = serde_json::json!({
					"code": code,
					"state": state
				});
				let _ = rx
					.send(Message::Text(serde_json::to_string(&data).unwrap()))
					.await;

				return (StatusCode::OK, "You may now close this tab");
			}
		}
	}
}

#[derive(Debug, Deserialize)]
struct MessageData {
	api_key: String,
	state: String,
}
