use std::{collections::HashMap, env, sync::Arc, time::Duration};

use anyhow::Result;
use futures_util::{
	stream::{self, SplitSink},
	SinkExt, StreamExt,
};
use serde::{Deserialize, Serialize};
use tokio::{
	runtime::Builder,
	sync::RwLock,
	time::{interval, timeout},
};
use warp::{
	hyper::StatusCode,
	ws::{Message, WebSocket},
	Filter,
};

#[derive(Clone, Serialize, Deserialize)]
struct MessageData {
	api_key: String,
	state: String,
}

// type Users = Arc<std::sync::RwLock<HashMap<String, oneshot::Sender<Message>>>>;
type Users = Arc<RwLock<HashMap<String, SplitSink<WebSocket, Message>>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u16)]
#[allow(dead_code)]
enum WsCloseCode {
	NormalClosure = 1000,
	GoingAway,
	ProtocolError,
	UnsupportedData,
	InvalidFramePayloadData = 1007,
	PolicyViolation,
	MessageTooBig,
	MandatoryExt,
	InternalError,
	ServiceRestart,
	TryAgainLater,
	InvalidResponse,
	Unauthorized = 3000,
}

impl From<WsCloseCode> for u16 {
	fn from(x: WsCloseCode) -> Self {
		x as Self
	}
}

fn main() -> Result<()> {
	dotenv::dotenv().ok();
	let rt = Builder::new_multi_thread().enable_all().build()?;

	rt.block_on(run())?;

	Ok(())
}

async fn run() -> Result<()> {
	let users = Users::default();

	let users = warp::any().map(move || users.clone());

	let socket =
		warp::path("socket")
			.and(warp::ws())
			.and(users.clone())
			.map(|ws: warp::ws::Ws, users| {
				ws.on_upgrade(move |websocket| user_connected(websocket, users))
			});

	let index = warp::path::end()
		.and(warp::query::<HashMap<String, String>>())
		.and(users)
		.then(|qs: HashMap<String, String>, users: Users| async move {
			let state = qs.get("state").map(String::as_str);
			let code = if let Some(c) = qs.get("code").map(String::as_str) {
				c
			} else {
				return warp::reply::with_status(
					"No code received from api",
					StatusCode::INTERNAL_SERVER_ERROR,
				);
			};

			match state {
				None => {
					return warp::reply::with_status(
						"No \"state\" parameter",
						StatusCode::BAD_REQUEST,
					)
				}
				Some(s) => match users.write().await.remove(s) {
					None => {
						return warp::reply::with_status(
							"\"state\" parameter not registered",
							StatusCode::BAD_REQUEST,
						)
					}
					Some(mut rx) => {
						let mut messages = stream::iter(
							[
								Message::text(code),
								Message::close_with(
									WsCloseCode::NormalClosure,
									"finished sending data",
								),
							]
							.map(Ok),
						);
						let _ = rx.send_all(&mut messages).await;
						let _ = rx.close().await;

						return warp::reply::with_status(
							"You may now close this tab",
							StatusCode::OK,
						);
					}
				},
			}
		});

	let routes = socket.or(index);

	warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

	Ok(())
}

async fn user_connected(mut ws: WebSocket, users: Users) {
	let first_message = if let Ok(Some(Ok(msg))) = timeout(Duration::from_secs(15), ws.next()).await
	{
		msg
	} else {
		let _ = ws
			.send(Message::close_with(
				WsCloseCode::TryAgainLater,
				"timed out waiting for auth message",
			))
			.await;
		let _ = ws.close().await;
		return;
	};

	if !first_message.is_text() {
		let _ = ws
			.send(Message::close_with(
				WsCloseCode::UnsupportedData,
				"Expected binary encoded values",
			))
			.await;
		let _ = ws.close().await;
		return;
	}

	let message_data = match serde_json::from_slice::<MessageData>(first_message.as_bytes()) {
		Err(_) => {
			let _ = ws
				.send(Message::close_with(
					WsCloseCode::UnsupportedData,
					"expected valid JSON data",
				))
				.await;
			let _ = ws.close().await;
			return;
		}
		Ok(d) => d,
	};

	// get the api key to check if the message sent was valid
	let api_key = env::var("API_KEY").expect("couldn't find api key");

	if message_data.api_key != api_key {
		let _ = ws
			.send(Message::close_with(
				WsCloseCode::Unauthorized,
				"api_key was invalid",
			))
			.await;
		let _ = ws.close().await;
		return;
	}

	let (user_ws_tx, mut user_ws_rx) = ws.split();

	users
		.write()
		.await
		.insert(message_data.state.clone(), user_ws_tx);

	let mut ping_interval = interval(Duration::from_secs(5));

	loop {
		tokio::select! {
			Some(Ok(msg)) = user_ws_rx.next() => {
				if msg.is_close() {
					users.write().await.remove(message_data.state.as_str());
					break;
				}
			}
			_ = ping_interval.tick() => {
				let ping = Message::ping(vec![]);
				if let Some(tx) = users.write().await.get_mut(message_data.state.as_str()) {
					let _ = tx.send(ping).await;
				}
			}
		}
	}
}
