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
        .unwrap()
        .iter()
        .map(Member)
        .collect();

    Ok(member_names)
}
