use std::{env, sync::Arc};

use anyhow::Result;
use axum::{routing::get, Extension, Router};
use fern::colors::{Color, ColoredLevelConfig};
use log::LevelFilter;
use tokio::runtime::Builder;

mod health_check;
mod main_page;
mod redirect;
mod retrieval;
mod state;

fn main() -> Result<()> {
	dotenv::dotenv().ok();

	let keys = env::var("API_KEYS")
		.expect("no API_KEYS env variable present")
		.split(",")
		.map(ToOwned::to_owned)
		.collect();

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

	rt.block_on(run(keys))?;

	Ok(())
}

async fn run(keys: Vec<String>) -> Result<()> {
	axum::Server::bind(&([0, 0, 0, 0], 3030).into())
		.serve(app(keys).into_make_service())
		.await?;

	Ok(())
}

fn app(keys: Vec<String>) -> Router {
	let state = Extension(Arc::new(state::State::new(keys)));
	Router::new()
		.route("/", get(main_page::handler))
		.route("/redirect", get(redirect::handler))
		.route("/retrieval", get(retrieval::handler))
		.route("/health-check", get(health_check::handler))
		.layer(state)
}
