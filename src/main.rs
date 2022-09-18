use anyhow::Result;
// use axum::{routing::get, Router, extract::WebSocketUpgrade};
use axum::{
	extract::ws::{WebSocket, WebSocketUpgrade},
	response::{IntoResponse, Response},
	routing::get,
	Router,
};
use fern::colors::{Color, ColoredLevelConfig};
use log::LevelFilter;
use tokio::runtime::Builder;

fn main() -> Result<()> {
	dotenv::dotenv().ok();

	let rt = Builder::new_multi_thread().enable_all().build()?;

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

	rt.block_on(run())?;

	Ok(())
}

async fn run() -> Result<()> {
	let app = Router::new()
		.route("/", get(|| async { "Hello, world!" }))
		.route("/ws", get(handler));

	axum::Server::bind(&([0, 0, 0, 0], 3000).into())
		.serve(app.into_make_service())
		.await?;

	Ok(())
}

async fn handler(ws: WebSocketUpgrade) -> Response {
	ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
	while let Some(msg) = socket.recv().await {
		let msg = if let Ok(msg) = msg {
			msg
		} else {
			return;
		};

		if socket.send(msg).await.is_err() {
			return;
		}
	}
}
