use crate::config::Config;
use crate::cookie_database::CookieDatabase;
use crate::membership::Membership;

mod bot;
mod config;
mod cookie_database;
mod membership;
mod scraper;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    if dotenv::dotenv().is_err() {
        log::warn!("Didn't find a .env file");
    }
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "bruce=info");
    }
    env_logger::init();
    let config = Config::generate();
    let conn = config.get_sqlite_conn();
    Membership::init_table(&conn);
    CookieDatabase::init_table(&conn);

    let scraper = tokio::spawn(scraper::run(config.clone()));
    let bot = tokio::spawn(bot::run(config.clone()));
    scraper.await.unwrap();
    bot.await.unwrap();
}
