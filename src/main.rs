use anyhow::Result;
use tokio::runtime::Builder;
use warp::Filter;

fn main() -> Result<()> {
	let rt = Builder::new_multi_thread().enable_all().build()?;

	rt.block_on(run())?;

	Ok(())
}

async fn run() -> Result<()> {
	let close_route = warp::any().map(|| "You may now close this window");

	warp::serve(close_route).run(([127, 0, 0, 1], 3030)).await;

	Ok(())
}

