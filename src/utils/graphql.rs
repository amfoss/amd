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

const REQUEST_URL: &str = "https://root.shuttleapp.rs/";

pub async fn fetch_members() -> Result<Vec<(String, i32)>, reqwest::Error> {
    let client = reqwest::Client::new();
    let query = r#"
    query {
        getMember {
            name
            id
        }
    }"#;

    let response = client
        .post(REQUEST_URL)
        .json(&serde_json::json!({"query": query}))
        .send()
        .await?;

    let json: Value = response.json().await?;

    let member_names: Vec<(String, i32)> = json["data"]["getMember"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|member| {
            let id = member["id"].as_i64().unwrap_or(0) as i32;
            let name = member["name"].as_str().unwrap_or("").to_string();
            (name, id)
        })
        .collect();

    Ok(member_names)
}

pub async fn send_streak_update(
    root_api_url: &str, 
    id: i32, 
    has_sent_update: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let query = r#"
        mutation updateStreak($id: Int!, $hasSentUpdate: Boolean!) {
            updateStreak(id: $id, hasSentUpdate: $hasSentUpdate) {
                id
                streak
                maxStreak
            }
        }
    "#;
    
    let variables = serde_json::json!({
        "id": id, 
        "hasSentUpdate": has_sent_update
    });
    
    let body = serde_json::json!({
        "query": query, 
        "variables": variables
    });
    
    let response = client
        .post(root_api_url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    let response_status = response.status();
    let response_body = response.text().await?;

    if response_status.is_success() {
        println!("Successfully updated streak for ID {}: {}", id, response_body);
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Failed to update streak for ID {}. HTTP status: {}, response: {}", 
            id, response_status, response_body
        ).into())
    }
}