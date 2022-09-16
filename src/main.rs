use std::{collections::HashMap, env, sync::Arc, time::Duration};

use anyhow::Result;
use fern::colors::{Color, ColoredLevelConfig};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use log::LevelFilter;
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
	dotenv::dotenv().ok(); // fallible, but we don't care if it does

	assert!(
		env::var("API_KEY").is_ok(),
		"no API_KEY env variable present"
	);

	let rt = Builder::new_multi_thread().enable_all().build()?;

	rt.block_on(run())?;

	Ok(())
}

async fn run() -> Result<()> {
	let colors = ColoredLevelConfig {
		error: Color::Red,
		warn: Color::Yellow,
		info: Color::White,
		debug: Color::Cyan,
		trace: Color::Magenta,
	};

	fern::Dispatch::new()
		.format(move |out, message, record| {
			out.finish(format_args!(
				"{}[{}][{}] {}",
				chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
				record.target(),
				colors.color(record.level()),
				message
			))
		})
		.level(LevelFilter::Debug)
		.chain(fern::Output::from(std::io::stderr()))
		.apply()?;

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
			let code = if let Some(c) = qs.get("code").cloned() {
				log::info!("received code {} from the api", c);
				c
			} else {
				log::error!("didn't receive code from the bungie api");
				return warp::reply::with_status(
					"No code received from api",
					StatusCode::INTERNAL_SERVER_ERROR,
				);
			};

			match state {
				None => {
					log::warn!("no state parameter sent from api redirect");
					return warp::reply::with_status(
						"No \"state\" parameter",
						StatusCode::BAD_REQUEST,
					);
				}
				Some(s) => match users.write().await.remove(s) {
					None => {
						log::warn!("state parameter not sent to websocket before redirect");
						return warp::reply::with_status(
							"\"state\" parameter not registered",
							StatusCode::BAD_REQUEST,
						);
					}
					Some(mut rx) => {
						log::info!("code {} successfully sent with state {}", code, s);
						let _ = rx.send(Message::text(code + ":" + s)).await;
						// rely on my application to send close code.

						return warp::reply::with_status(
							"You may now close this tab",
							StatusCode::OK,
						);
					}
				},
			}
		});

	let routes = socket.or(index);

	log::debug!(
		"server running with bungie api key {}",
		env::var("API_KEY")?
	);

	#[cfg(feature = "tls")]
	warp::serve(routes)
		.tls()
		.cert_path("localhost.pem")
		.key_path("localhost-key.pem")
		.run(([127, 0, 0, 1], 3030))
		.await;

	// don't use local certifications as we'll be going through nginx
	#[cfg(not(feature = "tls"))]
	warp::serve(routes).run(([0, 0, 0, 0], 3030)).await;

	Ok(())
}

async fn user_connected(mut ws: WebSocket, users: Users) {
	// let api_key = env::var("API_KEY").expect("no API_KEY set in process env");
	let api_key = match env::var("API_KEY") {
		Ok(v) => v,
		Err(_) => unsafe { std::hint::unreachable_unchecked() },
	};

	let first_message = if let Ok(Some(Ok(msg))) = timeout(Duration::from_secs(15), ws.next()).await
	{
		msg
	} else {
		log::error!("didn't receive auth message within 15 seconds, closing connection to");
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
		log::error!("auth message was not sent as text");
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
			log::error!("message data was not valid JSON");
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

	log::info!("got auth message from {}", message_data.state);

	if message_data.api_key != api_key {
		log::error!("api key didn't match set key from {}", message_data.state);
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
					log::info!("was sent close code, closing connection {}", message_data.state);
					users.write().await.remove(message_data.state.as_str());
					return;
				}
			}
			_ = ping_interval.tick() => {
				log::debug!("sending ping to connection {}", message_data.state);
				let ping = Message::ping(vec![]);
				if let Some(tx) = users.write().await.get_mut(message_data.state.as_str()) {
					let _ = tx.send(ping).await;
				}
			}
		}
	}
}
