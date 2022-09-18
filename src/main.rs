use std::{env, sync::Arc};

use anyhow::Result;
use axum::{routing::get, Extension, Router};
use fern::colors::{Color, ColoredLevelConfig};
use log::LevelFilter;
use tokio::runtime::Builder;

mod main_page;
mod params;
mod socket;
mod state;

fn main() -> Result<()> {
	dotenv::dotenv().ok();

	assert!(
		env::var("API_KEYS").is_ok(),
		"no API_KEYS env variable present"
	);

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
	axum::Server::bind(&([0, 0, 0, 0], 3030).into())
		.serve(app().into_make_service())
		.await?;

	Ok(())
}

fn app() -> Router {
	let state = Extension(Arc::new(state::State::default()));
	Router::new()
		.route("/", get(main_page::handler))
		.route("/redirect", get(params::handler))
		.route("/socket", get(socket::handler))
		.layer(state)
}
