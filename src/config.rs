use config::{Config, File};
use serde::Deserialize;
use lazy_static::lazy_static;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct Discord {
    pub roles_message_id: u64,
}

#[derive(Debug, Deserialize)]
pub struct Roles {
    pub archive: u64,
    pub mobile: u64,
    pub systems: u64,
    pub ai: u64,
    pub research: u64,
    pub devops: u64,
    pub web: u64,
}

#[derive(Debug, Deserialize)]
pub struct Channels {
    pub group_one: u64,
    pub group_two: u64,
    pub group_three: u64,
    pub group_four: u64,
    pub status_update: u64,
}

#[derive(Debug, Deserialize)]
pub struct StatusUpdate {
    pub title_url: String,
    pub image_url: String,
    pub author_url: String,
    pub icon_url: String,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub discord: Discord,
    pub roles: Roles,
    pub channels: Channels,
    pub status_update: StatusUpdate,
}

lazy_static! {
    pub static ref CONFIG: Arc<Settings> = {
        let config = Config::builder()
            .add_source(File::with_name("config.toml"))
            .build()
            .expect("Failed to load configuration")
            .try_deserialize()
            .expect("Invalid configuration format");

        Arc::new(config)
    };
}
