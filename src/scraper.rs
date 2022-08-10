use std::collections::HashSet;
use std::sync::Arc;

use crate::config::Config;
use crate::cookie_database::CookieDatabase;
use crate::error::Error;
use reqwest::{Client, StatusCode};
use scraper::Selector;

use crate::membership::Membership;

pub(crate) async fn init(config: Config) -> Result<(), Error> {
    let cookie_db = Arc::new(CookieDatabase::new(config.get_sqlite_conn()?));
    let client = Client::builder()
        .cookie_provider(cookie_db.clone())
        .build()?;
    let mut memberships = Err(Error::from("No memberships"));
    if let Ok(cookie) = cookie_db.get_cookie_value(&config.members_url) {
        log::info!("Trying saved cookie: {}", cookie);
        memberships = scrape_memberships(&config, &client).await;
    }
    if memberships.is_err() || memberships.as_ref().unwrap().is_empty() {
        log::info!("Trying initial cookie: {}", &config.initial_cookie_value);
        cookie_db.add_cookie(
            &config.members_url,
            "su_session",
            &config.initial_cookie_value,
        )?;
        memberships = scrape_memberships(&config, &client).await;
    }
    if let Err(err) = memberships {
        log::error!("{}", err);
        return Err(Error::from(
            "Failed to scrape members with known cookies, try obtaining another one",
        ));
    }
    run().await;
    Ok(())
}

pub(crate) async fn run() {
    let config = Config::generate();
    let cookie_jar = Arc::new(CookieDatabase::new(match config.get_sqlite_conn() {
        Ok(conn) => conn,
        Err(e) => {
            log::error!("{}", e);
            return;
        }
    }));
    let client = match Client::builder()
        .cookie_provider(cookie_jar.clone())
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            log::error!("{}", e);
            return;
        }
    };
    let memberships = match scrape_memberships(&config, &client).await {
        Ok(memberships) => memberships,
        Err(e) => {
            log::error!("{}", e);
            return;
        }
    };
    let conn = match config.get_sqlite_conn() {
        Ok(client) => client,
        Err(e) => {
            log::error!("{}", e);
            return;
        }
    };
    for mut membership in match Membership::get_all(&conn) {
        Ok(client) => client,
        Err(e) => {
            log::error!("{}", e);
            return;
        }
    } {
        let student_ids: HashSet<u32> = memberships.iter().map(|m| m.student_id).collect();
        if !student_ids.contains(&membership.student_id) {
            if membership.discord_id.is_none() {
                match membership.delete(&conn) {
                    Ok(client) => client,
                    Err(e) => {
                        log::error!("{}", e);
                        continue;
                    }
                };
            } else {
                match membership.update_should_drop(&conn, true) {
                    Ok(client) => client,
                    Err(e) => {
                        log::error!("{}", e);
                        continue;
                    }
                };
            }
        }
    }
    for membership in memberships {
        match membership.insert(&conn) {
            Ok(client) => client,
            Err(e) => {
                log::error!("{}", e);
                continue;
            }
        };
    }
}

async fn scrape_memberships(config: &Config, client: &Client) -> Result<Vec<Membership>, Error> {
    let request = client.get(config.members_url.clone()).build()?;
    let response = client.execute(request).await?;

    if response.status() != StatusCode::OK {
        return Err(Error::from(format!(
            "Failed to scrape members, status code: {}",
            response.status()
        )));
    }

    let html = response.text().await?;
    if html.contains("Sorry you're not authenticated") {
        return Err(Error::from(
            "Failed to scrape members, cookie not providing authenticated access",
        ));
    }
    let html = scraper::Html::parse_document(&html);
    let sel_tr = Selector::parse("#group-member-list-datatable > tbody > tr")
        .expect("Failed to create selector");
    let sel_td = Selector::parse("td").expect("Failed to create selector");
    let mut memberships = vec![];
    for tr in html.select(&sel_tr).map(|e| e.select(&sel_td)) {
        let data: Vec<Option<&str>> = tr.take(2).map(|td| td.text().next()).collect();
        memberships.push(Membership {
            student_id: data[0].ok_or("Unexpected td value")?.parse()?,
            name: data[1].ok_or("Unexpected td value")?.to_string(),
            discord_id: None,
            should_drop: false,
        });
    }

    log::info!("Scraped {} members", memberships.len());
    Ok(memberships)
}
