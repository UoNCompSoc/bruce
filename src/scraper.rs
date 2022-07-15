use std::collections::HashSet;
use std::sync::Arc;

use reqwest::cookie::{CookieStore, Jar};
use reqwest::{Client, StatusCode, Url};
use rusqlite::{params, Connection};
use scraper::Selector;

use crate::membership::Membership;

mod membership;

struct Config {
    members_url: Url,
    sqlite_file: String,
    initial_cookie_value: String,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("load .env");
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "scraper=info");
    }
    env_logger::init();
    let config = Config {
        members_url: std::env::var("MEMBERS_URL")
            .expect("MEMBERS_URL")
            .parse::<Url>()
            .expect("valid MEMBERS_URL"),
        sqlite_file: std::env::var("SQLITE_FILE").unwrap_or_else(|_| "db.sqlite".to_string()),
        initial_cookie_value: std::env::var("INITIAL_SUMS_COOKIE_VALUE")
            .expect("INITIAL_SUMS_COOKIE_VALUE"),
    };

    let conn = Connection::open(&config.sqlite_file).expect("failed to open sqlite connection");
    Membership::init_table(&conn);

    let cookie_jar = Arc::new(Jar::default());
    let client = Client::builder()
        .cookie_provider(cookie_jar.clone())
        .build()
        .unwrap();

    let mut memberships = vec![];
    if let Some(cookie) = get_saved_cookie(&conn) {
        log::info!("Trying saved cookie: {}", cookie);
        write_cookie_store(&config, &cookie_jar, cookie.as_str());
        memberships = scrape_memberships(&config, &conn, &client, &cookie_jar).await;
    }
    if memberships.is_empty() {
        log::info!("Trying initial cookie: {}", &config.initial_cookie_value);
        write_cookie_store(&config, &cookie_jar, &config.initial_cookie_value);
        memberships = scrape_memberships(&config, &conn, &client, &cookie_jar).await;
    }
    if memberships.is_empty() {
        panic!("Failed to scrape members with known cookies, try obtaining another one")
    }
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

async fn scrape_memberships(
    config: &Config,
    conn: &Connection,
    client: &Client,
    cookie_jar: &Jar,
) -> Vec<Membership> {
    let request = client.get(config.members_url.clone()).build().unwrap();
    let response = client.execute(request).await.unwrap();

    if response.status() != StatusCode::OK {
        log::error!(
            "Failed to scrape members, status code: {}",
            response.status()
        );
        return vec![];
    }

    let html = response.text().await.unwrap();
    if html.contains("Sorry you're not authenticated") {
        log::error!("Failed to scrape members, invalid cookie");
        return vec![];
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
    let cookie = read_cookie_store(config, cookie_jar);
    log::info!("Saving cookie: {}", cookie);
    set_saved_cookie(conn, cookie.as_str());

    memberships
}

fn read_cookie_store(config: &Config, cookie_jar: &Jar) -> String {
    cookie_jar
        .cookies(&config.members_url)
        .unwrap()
        .to_str()
        .unwrap()
        .split('=')
        .last()
        .unwrap()
        .to_string()
}

fn write_cookie_store(config: &Config, cookie_jar: &Jar, cookie_value: &str) {
    cookie_jar.add_cookie_str(
        format!("su_session={}", cookie_value).as_str(),
        &config.members_url,
    );
}

fn get_saved_cookie(conn: &Connection) -> Option<String> {
    let mut stmt = conn.prepare("SELECT value FROM cookie").unwrap();
    stmt.query_row(params![], |r| Ok(r.get(0).unwrap())).ok()
}

fn set_saved_cookie(conn: &Connection, cookie: &str) {
    conn.execute("DELETE FROM cookie", params![])
        .unwrap();
    conn.execute("INSERT INTO cookie (value) VALUES (?1)", params![cookie])
        .unwrap();
}
