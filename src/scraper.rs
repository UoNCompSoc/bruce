use std::collections::HashSet;
use std::sync::Arc;

use crate::config::Config;
use crate::cookie_database::CookieDatabase;
use reqwest::{Client, StatusCode};
use scraper::Selector;
use tokio_schedule::Job;

use crate::membership::Membership;

pub(crate) async fn run(config: Config) {
    log::info!("Scraper starting");
    let cookie_db = Arc::new(CookieDatabase::new(config.get_sqlite_conn()));
    let client = Client::builder()
        .cookie_provider(cookie_db.clone())
        .build()
        .unwrap();

    let mut memberships = None;
    if let Some(cookie) = cookie_db.get_cookie_value(&config.members_url) {
        log::info!("Trying saved cookie: {}", cookie);
        memberships = scrape_memberships(&config, &client).await;
    }
    if memberships.is_none() {
        log::info!("Trying initial cookie: {}", &config.initial_cookie_value);
        cookie_db.add_cookie(
            &config.members_url,
            "su_session",
            &config.initial_cookie_value,
        );
        memberships = scrape_memberships(&config, &client).await;
    }
    if memberships.is_none() {
        panic!("Failed to scrape members with known cookies, try obtaining another one")
    }
    local_run().await;
    let schedule = tokio_schedule::every(2).hours().perform(local_run);
    tokio::spawn(schedule).await.expect("keep running");
}

// TODO: Make this the run function instead and start the schedule from main
pub(crate) async fn local_run() {
    let config = Config::generate();
    let cookie_jar = Arc::new(CookieDatabase::new(config.get_sqlite_conn()));
    let client = Client::builder()
        .cookie_provider(cookie_jar.clone())
        .build()
        .unwrap();
    if let Some(memberships) = scrape_memberships(&config, &client).await {
        let conn = config.get_sqlite_conn();
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
        log::error!("Failed to scrape members, cookie not providing authenticated access");
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
