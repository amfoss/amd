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

use crate::graphql::models::{
    AttendanceRecord, LeaveCountRecord, LeaveRecordWithMessage, Member, MemberSummary,
};

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

    pub async fn fetch_member_summary(
        &self,
        discord_id: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> anyhow::Result<MemberSummary> {
        let query: &str = r#"
        query($discord_id: String!, $start_date: NaiveDate!, $end_date: NaiveDate!){
            member(discordId : $discord_id){
                attendance{
                    presentCount(startDate : $start_date,endDate : $end_date)
                    absentCount(startDate : $start_date,endDate : $end_date)
                }
                status{
                    updateCount(startDate : $start_date,endDate : $end_date)
                }
            }
        }
        "#;

        let variables = serde_json::json!({
            "start_date": start_date.format("%Y-%m-%d").to_string(),
            "end_date": end_date.format("%Y-%m-%d").to_string(),
            "discord_id": discord_id
        });

        debug!("Sending query {}", query);
        debug!("With variables {:?}", variables);

        let response = self
            .http()
            .post(self.root_url())
            .bearer_auth(self.api_key())
            .json(&serde_json::json!({ "query": query , "variables":variables}))
            .send()
            .await
            .context("Failed to send GraphQL request")?;
        debug!("Response status: {:?}", response.status());

        let json: Value = response
            .json()
            .await
            .context("Failed to parse response as JSON")?;

        debug!("Response JSON: {:#?}", json);

        let attendance = &json["data"]["member"]["attendance"];
        let status = &json["data"]["member"]["status"];

        let present: i32 = attendance["presentCount"].as_i64().unwrap_or(0) as i32;
        let absent: i32 = attendance["absentCount"].as_i64().unwrap_or(0) as i32;
        let updates: i32 = status["updateCount"].as_i64().unwrap_or(0) as i32;

        let total_attendance = present + absent;

        let attendance_percent = if total_attendance == 0 {
            0.0
        } else {
            (present as f32 * 100.0) / total_attendance as f32
        };

        let total_days = (end_date - start_date).num_days() + 1;

        if total_days < 0 {
            return Err(anyhow!("end_date must be on/after start_date"));
        }

        let total_days = (total_days + 1).max(1) as f32;

        let update_percent = (updates as f32 * 100.0) / total_days;

        let summary = MemberSummary {
            present_percent: attendance_percent,
            updates_percent: update_percent,
        };

        Ok(summary)
    }

    pub async fn fetch_leaves(
        &self,
        discord_id: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> anyhow::Result<LeaveCountRecord> {
        let query = r#" 
            query ($discord_id: String!, $start_date: String!, $end_date: String!) {
            member(discordId :$discord_id ) {
                leaveCount(startDate: $start_date,endDate: $end_date)
            }
            }
        "#;

        let variables = serde_json::json!({
            "discord_id": discord_id,
            "start_date": start_date.format("%Y-%m-%d").to_string(),
            "end_date": end_date.format("%Y-%m-%d").to_string(),
        });

        debug!("Sending query {}", query);
        debug!("With variables {:?}", variables);

        let response = self
            .http()
            .post(self.root_url())
            .bearer_auth(self.api_key())
            .json(&serde_json::json!({
                "query": query,
                "variables": variables
            }))
            .send()
            .await
            .context("Failed to send GraphQL request")?;

        debug!("Response status: {:?}", response.status());

        let json: Value = response
            .json()
            .await
            .context("Failed to parse response as JSON")?;

        let leaves: LeaveCountRecord = LeaveCountRecord {
            discord_id: discord_id.to_string(),
            leave_count: json["data"]["member"]["leaveCount"].as_i64().unwrap_or(0) as i32,
        };

        Ok(leaves)
    }

    pub async fn check_leave(
        &self,
        message_id: u64
    ) -> anyhow::Result<LeaveRecordWithMessage> {
        let query = r#"
            query($message_id: String!) {
                leaveByMessageId(
                    messageId: $message_id
                ) {
                    discordId
                    fromDate
                    duration
                    messageId
                    approvedBy
                    appliedAt
                }
            }
        "#;

        let variables = serde_json::json!({
            "message_id": message_id.to_string()
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
            .await?;

        let json: serde_json::Value = response.json().await?;

        if let Some(errors) = json.get("errors") {
            anyhow::bail!("GraphQL errors: {:#}", errors);
        }

        let leave_value = json
            .get("data")
            .and_then(|data| data.get("leaveByMessageId"))
            .ok_or_else(|| anyhow::anyhow!("Missing data.leaveByMessageId"))?;

        let leave: LeaveRecordWithMessage = serde_json::from_value(leave_value.clone())?;

        Ok(leave)
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
}
