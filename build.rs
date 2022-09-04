use std::env;

fn main() {
    dotenv::dotenv().ok();
    let api_key = env::var("API_KEY").unwrap();

    println!("cargo:rustc-env=API_KEY={}", api_key);
}