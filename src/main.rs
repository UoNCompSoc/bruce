use crate::config::Config;
use crate::cookie_database::CookieDatabase;
use crate::membership::Membership;
use tokio_schedule::Job;

mod bot;
mod config;
mod cookie_database;
mod error;
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
    let conn = match config.get_sqlite_conn() {
        Ok(client) => client,
        Err(e) => {
            log::error!("{}", e);
            return;
        }
    };
    Membership::init_table(&conn).expect("initialize membership table");
    CookieDatabase::init_table(&conn).expect("initialize cookie table");

    scraper::init(config.clone())
        .await
        .expect("initialize scraper");
    let bot = tokio::spawn(bot::build_framework(config.clone()).run());
    while let Err(err) = tokio::spawn(tokio_schedule::every(2).hours().perform(scraper::run)).await
    {
        log::error!("{}", err);
    }
    bot.await.expect("bot running").expect("bot running");
}
