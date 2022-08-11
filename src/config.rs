use anyhow::Result;
use poise::serenity_prelude::Http;
use reqwest::Url;
use rusqlite::Connection;
use std::fs::OpenOptions;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Config {
    pub members_url: Url,
    pub data_dir: String,
    pub initial_cookie_value: String,
    pub discord_token: String,
    pub member_role_name: String,
    pub privileged_role_name: String,
    pub student_id_length: usize,
    pub membership_purchase_url: Option<String>,
}

impl Config {
    pub fn generate() -> Self {
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

    pub fn get_http(&self) -> Http {
        Http::new(self.discord_token.as_str())
    }

    pub fn get_sqlite_file(&self) -> PathBuf {
        let mut file = PathBuf::from(&self.data_dir);
        file.push("db");
        file.set_extension("sqlite");
        file
    }

    pub fn get_sqlite_conn(&self) -> Result<Connection> {
        let file = self.get_sqlite_file();
        OpenOptions::new().create(true).write(true).open(&file)?;
        Ok(Connection::open(&file)?)
    }
}
