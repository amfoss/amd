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
use anyhow::{anyhow, Context};
use chrono::{Local, NaiveDate};
use serde_json::Value;
use tracing::debug;

use crate::graphql::models::{AttendanceRecord, LifeStatus, Member};

use super::GraphQLClient;

impl GraphQLClient {
    pub async fn fetch_member_data(&self, date: NaiveDate) -> anyhow::Result<Vec<Member>> {
        let query = r#"
        query($date: NaiveDate!) {
          allMembers {
            memberId
            name
            discordId
            groupId
            status {
              onDate(date: $date) {
                isSent
                onBreak
              }
              streak {
                currentStreak,
                maxStreak
              }
              consecutiveMisses
              lifeStatus {
                memberId
                lives
                recoveryStreak
                isProbation
                lastResetMonth
              }
            }
            track
            year
            email
          }
        }"#;

        debug!("Sending query {}", query);

        let variables = serde_json::json!({
            "date": date.format("%Y-%m-%d").to_string()
        });

        debug!("With variables: {}", variables);

        let response = self
            .http()
            .post(self.root_url())
            .bearer_auth(self.api_key())
            .json(&serde_json::json!({"query": query, "variables":variables}))
            .send()
            .await
            .context("Failed to successfully post request")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Server responded with an error: {:?}",
                response.status()
            ));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .context("Failed to serialize response")?;

        debug!("Response: {}", response_json);
        let members = response_json
            .get("data")
            .and_then(|data| data.get("allMembers"))
            .and_then(|members| members.as_array())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Malformed response: Could not access Members from {}",
                    response_json
                )
            })?;

        let members: Vec<Member> =
            serde_json::from_value(serde_json::Value::Array(members.clone()))
                .context("Failed to parse 'members' into Vec<Member>")?;

        Ok(members)
    }

    pub async fn fetch_attendance(&self) -> anyhow::Result<Vec<AttendanceRecord>> {
        debug!("Fetching attendance data");

        let today = Local::now().format("%Y-%m-%d").to_string();
        let query = format!(
            r#"
        query {{
            attendanceByDate(date: "{today}") {{
                name,
                year,
                isPresent,
                timeIn,
            }}
        }}"#
        );

        let response = self
            .http()
            .post(self.root_url())
            .bearer_auth(self.api_key())
            .json(&serde_json::json!({ "query": query }))
            .send()
            .await
            .context("Failed to send GraphQL request")?;
        debug!("Response status: {:?}", response.status());

        let json: Value = response
            .json()
            .await
            .context("Failed to parse response as JSON")?;

        let attendance_array = json["data"]["attendanceByDate"]
            .as_array()
            .context("Missing or invalid 'data.attendanceByDate' array in response")?;

        let attendance: Vec<AttendanceRecord> = attendance_array
            .iter()
            .map(|entry| {
                serde_json::from_value(entry.clone()).context("Failed to parse attendance record")
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        debug!(
            "Successfully fetched {} attendance records",
            attendance.len()
        );
        Ok(attendance)
    }

    pub async fn save_member_roles(
        &self,
        discord_id: String,
        roles: Vec<String>,
    ) -> anyhow::Result<()> {
        let query = r#"
        mutation($discordId: String!, $roles: [String!]!) {
            saveMemberRoles(discordId: $discordId, roles: $roles)
        }"#;

        let variables = serde_json::json!({
            "discordId": discord_id,
            "roles": roles
        });

        let res = self
            .http()
            .post(self.root_url())
            .bearer_auth(self.api_key())
            .json(&serde_json::json!({
                "query": query,
                "variables": variables
            }))
            .send()
            .await?
            .error_for_status()?;

        let response: serde_json::Value = res.json().await?;

        if response.get("errors").is_some() {
            anyhow::bail!("GraphQL error: {:?}", response["errors"]);
        }
        Ok(())
    }

    pub async fn get_member_roles(
        &self,
        discord_id: String,
    ) -> anyhow::Result<(bool, Vec<String>)> {
        let query = r#"
        query($discordId: String!) {
            memberRoles(discordId: $discordId){
                exists
                roles
            }
        }"#;

        let variables = serde_json::json!({
            "discordId": discord_id
        });

        let response = self
            .http()
            .post(self.root_url())
            .bearer_auth(self.api_key())
            .json(&serde_json::json!({
                "query": query,
                "variables": variables
            }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        // Collect roles returned by the API as strings; any filtering (e.g. removing @everyone) is handled by the caller.
        let data = &response["data"]["memberRoles"];

        let exists = data["exists"].as_bool().unwrap_or(false);

        let roles = data["roles"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();
        Ok((exists, roles))
    }

    pub async fn update_life_status(
        &self,
        member_id: i32,
        lives: i32,
        recovery_streak: i32,
        is_probation: bool,
        last_reset_month: i32,
    ) -> anyhow::Result<LifeStatus> {
        let query = r#"
        mutation($memberId: Int!, $lives: Int!, $recoveryStreak: Int!, $isProbation: Boolean!, $lastResetMonth: Int!) {
            updateLifeStatus(input: {
                memberId: $memberId
                lives: $lives
                recoveryStreak: $recoveryStreak
                isProbation: $isProbation
                lastResetMonth: $lastResetMonth
            }) {
                memberId
                lives
                recoveryStreak
                isProbation
                lastResetMonth
            }
        }"#;

        let variables = serde_json::json!({
            "memberId": member_id,
            "lives": lives,
            "recoveryStreak": recovery_streak,
            "isProbation": is_probation,
            "lastResetMonth": last_reset_month,
        });

        let response = self
            .http()
            .post(self.root_url())
            .bearer_auth(self.api_key())
            .json(&serde_json::json!({
                "query": query,
                "variables": variables
            }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        if response.get("errors").is_some() {
            anyhow::bail!("GraphQL error: {:?}", response["errors"]);
        }

        let data = response
            .get("data")
            .and_then(|data| data.get("updateLifeStatus"))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Malformed response: Could not access updateLifeStatus from {}",
                    response
                )
            })?;

        let status: LifeStatus = serde_json::from_value(data.clone())?;
        Ok(status)
    }
}
