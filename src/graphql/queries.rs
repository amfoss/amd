/*
amFOSS Daemon: A discord bot for the amFOSS Discord server.
Copyright (C) 2024 amFOSS

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
use serde_json::Value;
use anyhow::{Result};

use super::models::Member;

const REQUEST_URL: &str = "https://root.shuttleapp.rs/";

pub async fn fetch_members() -> Result<Vec<String>, reqwest::Error> {
    let client = reqwest::Client::new();
    let query = r#"
    query {
        getMember {
            name,
            groupId,
            discordId
        }
    }"#;

    let response = client
        .post(REQUEST_URL)
        .json(&serde_json::json!({"query": query}))
        .send()
        .await?;

    let json: Value = response.json().await?;

    let member_names: Vec<String> = json["data"]["getMember"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(Member)
        .collect();

    Ok(member_names)
}

pub async fn send_streak_update(
    root_api_url: &str,
    id: i32,
    has_sent_update: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let query = format!(
        r#"
        mutation {{
            updateStreak(id: {}, hasSentUpdate: {}) {{
                id
                streak
                max_streak
            }}
        }}
        "#,
        id, has_sent_update
    );
    let response = client
        .post(root_api_url)
        .header("Content-Type", "application/json")
        .body(query)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Successfully updated streak for ID {}", id);
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Failed to update streak for ID {}. HTTP status: {}",
            id,
            response.status()
        )
        .into())
    }
}