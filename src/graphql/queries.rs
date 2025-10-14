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

use crate::graphql::models::{AttendanceRecord, Member};

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
            }
            track
            year
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
}
