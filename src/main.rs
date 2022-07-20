use crate::config::Config;

mod bot;
mod config;
mod cookie_database;
mod membership;
mod scraper;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    dotenv::dotenv().expect("failed to load .env");
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "bruce=info");
    }
    env_logger::init();
    let config = Config::generate();

    let scraper = tokio::spawn(scraper::run(config.clone()));
    let bot = tokio::spawn(bot::run(config.clone()));
    scraper.await.unwrap();
    bot.await.unwrap();
}
