use std::collections::HashSet;
use std::sync::Arc;

use crate::cookie_database::CookieDatabase;
use reqwest::header::HeaderValue;
use reqwest::{Client, StatusCode, Url};
use rusqlite::Connection;
use scraper::Selector;
use tokio_schedule::Job;

use crate::membership::Membership;

mod cookie_database;
mod membership;

#[derive(Clone)]
struct Config {
    members_url: Url,
    sqlite_file: String,
    initial_cookie_value: String,
}

impl Config {
    fn generate() -> Self {
        Self {
            members_url: std::env::var("MEMBERS_URL")
                .expect("MEMBERS_URL")
                .parse::<Url>()
                .expect("valid MEMBERS_URL"),
            sqlite_file: std::env::var("SQLITE_FILE").unwrap_or_else(|_| "db.sqlite".to_string()),
            initial_cookie_value: std::env::var("INITIAL_SUMS_COOKIE_VALUE")
                .expect("INITIAL_SUMS_COOKIE_VALUE"),
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("load .env");
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "scraper=info");
    }
    env_logger::init();
    let config = Config::generate();

    let conn = Connection::open(&config.sqlite_file).expect("failed to open sqlite connection");
    Membership::init_table(&conn);

    let cookie_jar = Arc::new(CookieDatabase::new(config.sqlite_file.as_str()));
    let client = Client::builder()
        .cookie_provider(cookie_jar.clone())
        .build()
        .unwrap();

    let mut memberships = None;
    if let Some(cookie) = cookie_jar.get_cookie(&config.members_url) {
        log::info!("Trying saved cookie: {}", cookie);
        memberships = scrape_memberships(&config, &client).await;
    }
    if memberships.is_none() {
        log::info!("Trying initial cookie: {}", &config.initial_cookie_value);
        cookie_jar.add_cookie(
            &config.members_url,
            "su_session",
            &config.initial_cookie_value,
        );
        memberships = scrape_memberships(&config, &client).await;
    }
    if memberships.is_none() {
        panic!("Failed to scrape members with known cookies, try obtaining another one")
    }
    run().await;
    let schedule = tokio_schedule::every(30).seconds().perform(run);
    tokio::spawn(schedule).await.expect("keep running");
}

async fn run() {
    let config = Config::generate();
    let cookie_jar = Arc::new(CookieDatabase::new(&config.sqlite_file));
    let client = Client::builder()
        .cookie_provider(cookie_jar.clone())
        .build()
        .unwrap();
    if let Some(memberships) = scrape_memberships(&config, &client).await {
        let conn = Connection::open(&config.sqlite_file).expect("failed to open sqlite connection");
        for mut membership in Membership::get_all(&conn) {
            let student_ids: HashSet<u32> = memberships.iter().map(|m| m.student_id).collect();
            if !student_ids.contains(&membership.student_id) {
                if membership.discord_id.is_none() {
                    membership.delete(&conn);
                } else {
                    membership.update_should_drop(&conn, true);
                }
            }
        }
        for membership in memberships {
            membership.insert(&conn);
        }
    }
}

async fn scrape_memberships(config: &Config, client: &Client) -> Option<Vec<Membership>> {
    let request = client.get(config.members_url.clone()).build().unwrap();
    let response = client.execute(request).await.ok()?;

    if response.status() != StatusCode::OK {
        log::error!(
            "Failed to scrape members, status code: {}",
            response.status()
        );
        return None;
    }

    let html = response.text().await.unwrap();
    if html.contains("Sorry you're not authenticated") {
        log::error!("Failed to scrape members, invalid cookie");
        return None;
    }
    let html = scraper::Html::parse_document(&html);
    let sel_tr = Selector::parse("#group-member-list-datatable > tbody > tr").unwrap();
    let sel_td = Selector::parse("td").unwrap();
    let mut memberships = vec![];
    for tr in html.select(&sel_tr).map(|e| e.select(&sel_td)) {
        let data: Vec<&str> = tr.take(2).map(|td| td.text().next().unwrap()).collect();
        memberships.push(Membership {
            student_id: data[0].parse().unwrap(),
            name: data[1].to_string(),
            discord_id: None,
            should_drop: false,
        });
    }

    log::info!("Scraped {} members", memberships.len());
    Some(memberships)
}
