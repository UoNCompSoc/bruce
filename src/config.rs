use crate::error::Error;
use poise::serenity_prelude::Http;
use reqwest::Url;
use rusqlite::Connection;
use std::fs::OpenOptions;
use std::path::PathBuf;

#[derive(Clone)]
pub(crate) struct Config {
    pub(crate) members_url: Url,
    pub(crate) data_dir: String,
    pub(crate) initial_cookie_value: String,
    pub(crate) discord_token: String,
    pub(crate) member_role_name: String,
    pub(crate) privileged_role_name: String,
    pub(crate) student_id_length: usize,
    pub(crate) membership_purchase_url: Option<String>,
}

impl Config {
    pub(crate) fn generate() -> Self {
        Self {
            members_url: std::env::var("MEMBERS_URL")
                .expect("MEMBERS_URL")
                .parse::<Url>()
                .expect("valid MEMBERS_URL"),
            data_dir: std::env::var("DATA_DIR").unwrap_or_else(|_| "/data".to_string()),
            initial_cookie_value: std::env::var("INITIAL_SUMS_COOKIE_VALUE")
                .expect("INITIAL_SUMS_COOKIE_VALUE"),
            discord_token: std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN"),
            member_role_name: std::env::var("MEMBER_ROLE_NAME")
                .unwrap_or_else(|_| "Member".to_string()),
            privileged_role_name: std::env::var("PRIVILEGED_ROLE_NAME")
                .unwrap_or_else(|_| "Committee".to_string()),
            student_id_length: std::env::var("STUDENT_ID_LENGTH")
                .unwrap_or_else(|_| 8.to_string())
                .parse()
                .expect("Failed to parse STUDENT_ID_LENGTH as number"),
            membership_purchase_url: std::env::var("MEMBERSHIP_PURCHASE_URL").ok(),
        }
    }

    pub(crate) fn get_http(&self) -> Http {
        Http::new(self.discord_token.as_str())
    }

    pub(crate) fn get_sqlite_file(&self) -> PathBuf {
        let mut file = PathBuf::from(&self.data_dir);
        file.push("db");
        file.set_extension("sqlite");
        file
    }

    pub(crate) fn get_sqlite_conn(&self) -> Result<Connection, Error> {
        let file = self.get_sqlite_file();
        OpenOptions::new().create(true).write(true).open(&file)?;
        Ok(Connection::open(&file)?)
    }
}
