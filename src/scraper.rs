use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use reqwest::{Client, Url};
use reqwest::cookie::{CookieStore, Jar};
use rusqlite::{Connection, params};
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
    conn.execute(
        "CREATE TABLE IF NOT EXISTS cookie (value VARCHAR, timestamp DATETIME)",
        params![],
    )
    .unwrap();

    let cookie_jar = Arc::new(Jar::default());
    let client = Client::builder()
        .cookie_provider(cookie_jar.clone())
        .build()
        .unwrap();

    cookie_jar.add_cookie_str(
        format!("su_session={}", &config.initial_cookie_value).as_str(),
        &config.members_url,
    );

    let members = get_memberships(&config, &client, &cookie_jar).await;
}

async fn get_memberships(config: &Config, client: &Client, cookie_jar: &Jar) -> Vec<Membership> {
    let request = client.get(config.members_url.clone()).build().unwrap();
    let response = client.execute(request).await.unwrap();

    let x = cookie_jar
        .cookies(&config.members_url)
        .unwrap()
        .to_str()
        .unwrap()
        .split('=')
        .last()
        .unwrap()
        .to_string();
    println!("{}", response.text().await.unwrap());
    println!("{:?}", x);
    vec!()
}

fn get_cookie(conn: Connection) -> Option<(String, DateTime<Utc>)> {
    let mut stmt = conn.prepare("SELECT value, timestamp FROM cookie").unwrap();
    stmt.query_row(params![], |r| Ok((r.get(1).unwrap(), r.get(2).unwrap())))
        .ok()
}

fn set_cookie(conn: Connection, cookie: &str) {
    conn.execute("UPDATE cookie SET value = ?1", params![cookie])
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
