use std::collections::HashSet;
use std::sync::Arc;

use reqwest::cookie::{CookieStore, Jar};
use reqwest::{Client, StatusCode, Url};
use rusqlite::{params, Connection};

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
        std::env::set_var("RUST_LOG", "bruce=info");
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
                membership.drop(&conn);
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

    // log::info!("Scraped {} members", members.len());
    let cookie = read_cookie_store(config, cookie_jar);
    log::info!("Latest cookie: {}", cookie);
    set_saved_cookie(conn, cookie.as_str());
    vec![]
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
    let mut stmt = conn.prepare("SELECT value, timestamp FROM cookie").unwrap();
    stmt.query_row(params![], |r| Ok(r.get(1).unwrap())).ok()
}

fn set_saved_cookie(conn: &Connection, cookie: &str) {
    conn.execute(
        "UPDATE cookie SET value = ?1, timestamp = ?2",
        params![cookie],
    )
    .unwrap();
}

// protected List<Member> ScrapeHtml(StreamReader s)
// {
// var src = s.ReadToEnd();
// var html = HDocument.Parse(src);
//
// var rs = html.CssSelect("#group-member-list-datatable > tbody > tr");
// var ds = rs.Select(r => r.Children.Select(vs => vs.InnerText));
//
// var ms = new List<Member>();
//
// foreach (var r in ds)
// {
// var x = r.ToList();
//
// if (x.Count != 11)
// {
// foreach (var z in x)
// {
// Console.WriteLine(z.Trim());
// }
// Console.WriteLine();
// continue;
// }
//
// var m = new Member(Convert.ToUInt32(x[1].Trim()), x[3].Trim(), x[5].Trim(), x[9].Trim());
// ms.Add(m);
// }
//
// return ms;
// }
