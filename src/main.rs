use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use futures_util::{FutureExt, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{
	runtime::Builder,
	sync::{mpsc, RwLock},
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::{
	hyper::StatusCode,
	path::{FullPath, Tail},
	reply::Response,
	ws::{Message, WebSocket},
	Filter,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct MessageData {
	api_key: String,
	state: String,
}

type Users = Arc<std::sync::RwLock<HashMap<String, mpsc::UnboundedSender<Message>>>>;

fn main() -> Result<()> {
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

	let index = warp::path::full()
		.and(warp::query::<HashMap<String, String>>())
		.and(users)
		.map(
			|path: FullPath, qs: HashMap<String, String>, users: Users| {
				dbg!(&path);
				let state = qs.get("state");

				match state {
					None => {
						return warp::reply::with_status(
							"No \"state\" parameter",
							StatusCode::BAD_REQUEST,
						)
					}
					Some(s) => match users.read().unwrap().get(s) {
						None => {
							return warp::reply::with_status(
								"\"state\" parameter not registered",
								StatusCode::BAD_REQUEST,
							)
						}
						Some(sender) => {
							sender
								.send(Message::binary(path.as_str().as_bytes()))
								.unwrap();
							return warp::reply::with_status(
								"You may now close this tab",
								StatusCode::OK,
							);
						}
					},
				}
			},
		);

	let routes = index.or(socket);

	warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

	Ok(())
}

async fn user_connected(mut ws: WebSocket, users: Users) {
	let message = ws.next().await.unwrap().unwrap();

	let message_data = match serde_json::from_slice::<MessageData>(message.as_bytes()) {
		Err(_) => {
			ws.send(Message::close()).await.unwrap();
			ws.close().await.unwrap();
			return;
		}
		Ok(d) => d,
	};
}
