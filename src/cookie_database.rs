use std::string::String;

use fallible_iterator::FallibleIterator;
use reqwest::cookie::CookieStore;
use reqwest::header::HeaderValue;
use reqwest::Url;
use rusqlite::{params, Connection};
use crate::error::Error;

pub struct CookieDatabase {
    conn: Connection,
}

impl CookieDatabase {
    pub fn init_table(conn: &Connection) -> Result<(), Error> {
        conn.execute("CREATE TABLE IF NOT EXISTS cookies (url VARCHAR NOT NULL PRIMARY KEY, name VARCHAR NOT NULL, value VARCHAR NOT NULL)", params![])?;
        Ok(())
    }

    pub fn new(conn: Connection) -> Self {
        Self {
            conn,
        }
    }

    pub fn add_cookie<T: Into<String>>(&self, url: &Url, key: T, value: T) -> Result<(), Error> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO cookies (url, name, value) VALUES (?1, ?2, ?3)",
                params![url.to_string(), key.into(), value.into()],
            )?;
        Ok(())
    }

    pub fn get_cookie_value(&self, url: &Url) -> Result<String, Error> {
        Ok(self.conn
            .query_row(
                "SELECT value FROM cookies WHERE url = ?1",
                params![url.to_string()],
                |r| r.get(0),
            )?)
    }
}

unsafe impl Sync for CookieDatabase {}

impl CookieStore for CookieDatabase {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        for header in cookie_headers {
            let header = header
                .to_str().unwrap_or_else(|_| panic!("converting header to str {:?}", header));
            log::info!("Storing header: {}", header);
            let mut header = header
                .split(';')
                .next()
                .expect("getting key value pair from header")
                .split('=');
            if let Err(err) = self.add_cookie(
                url,
                header.next().expect("getting key from header"),
                header.next().expect("getting value from header"),
            ) {
                log::error!("{}", err);
            }
        }
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        HeaderValue::from_str(
            &self
                .conn
                .prepare("SELECT name, value FROM cookies WHERE url = ?1")
                .expect("preparing cookie fetch statement")
                .query(params![url.as_str()])
                .expect("running cookie fetch query")
                .map(|r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                .map(|(name, value)| Ok(format!("{}={};", name, value)))
                .collect::<Vec<String>>()
                .expect("gathering cookies into header string")
                .join(";"),
        )
        .ok()
    }
}

#[cfg(test)]
mod tests {
    use crate::cookie_database::CookieDatabase;
    use reqwest::cookie::CookieStore;
    use reqwest::header::HeaderValue;
    use reqwest::Url;
    use std::env::temp_dir;
    use rusqlite::Connection;

    #[test]
    fn in_out() {
        let mut test_db = temp_dir();
        test_db.push("test");
        test_db.set_extension("db");
        std::fs::remove_file(&test_db).unwrap_or(());
        let db = CookieDatabase::new(Connection::open(test_db).unwrap());

        let url = Url::parse("https://test.com").unwrap();
        let header_values = vec![HeaderValue::from_str("test=1234").unwrap()]; // dyn Iterator<Item = &HeaderValue>
        db.set_cookies(&mut header_values.iter(), &url);
        let output = db.cookies(&url);
        assert!(output.is_some());
        assert_eq!(output.unwrap().to_str().unwrap(), "test=1234;");
    }
}
