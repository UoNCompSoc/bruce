use std::string::String;

use fallible_iterator::FallibleIterator;
use reqwest::cookie::CookieStore;
use rusqlite::{params, Connection};

use crate::{HeaderValue, Url};

pub struct CookieDatabase {
    conn: Connection,
}

impl CookieDatabase {
    pub fn new(sqlite_file: &str) -> Self {
        let conn = Connection::open(sqlite_file).expect("failed to open sqlite connection");
        conn.execute("CREATE TABLE IF NOT EXISTS cookies (url VARCHAR NOT NULL PRIMARY KEY, name VARCHAR NOT NULL, value VARCHAR NOT NULL)", params![]).unwrap();
        Self { conn }
    }

    pub fn add_cookie<T: Into<String>>(&self, url: &Url, key: T, value: T) {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO cookies (url, name, value) VALUES (?1, ?2, ?3)",
                params![url.to_string(), key.into(), value.into()],
            )
            .unwrap();
    }

    pub fn get_cookie_value(&self, url: &Url) -> Option<String> {
        self.conn
            .query_row(
                "SELECT name, value FROM cookies WHERE url = ?1",
                params![url.to_string()],
                |r| r.get(0),
            )
            .ok()
    }
}

unsafe impl Sync for CookieDatabase {}

impl CookieStore for CookieDatabase {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        for header in cookie_headers {
            let header = header.to_str().unwrap();
            log::info!("Storing header: {}", header);
            let mut header = header.split(';').next().unwrap().split('=');
            self.add_cookie(url, header.next().unwrap(), header.next().unwrap());
        }
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, value FROM cookies WHERE url = ?1")
            .unwrap();
        let vec: Vec<(String, String)> = stmt
            .query(params![url.as_str()])
            .unwrap()
            .map(|r| Ok((r.get(0).unwrap(), r.get(1).unwrap())))
            .collect()
            .unwrap();
        let mut s = String::new();
        for (name, value) in vec.iter() {
            s.push_str(name);
            s.push('=');
            s.push_str(value);
        }
        HeaderValue::from_str(&s).ok()
    }
}
