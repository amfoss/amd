use crate::graphql::models::LeaveRecord;
use anyhow::Context;
use serde_json::Value;
use tracing::debug;

use super::GraphQLClient;
use chrono::NaiveDate;

impl GraphQLClient {
    pub async fn apply_leave(
        &self,
        discord_id: &str,
        message_id: &u64,
        start_date: NaiveDate,
        duration: i32,
        reason: &str,
    ) -> anyhow::Result<LeaveRecord> {
        let query = r#"
            mutation($discord_id: String!, $start_date: String!, $duration: Int!, $reason: String, $message_id: String) {
            leaveApplication(
                discordId: $discord_id,
                fromDate: $start_date,
                duration: $duration,
                reason: $reason,
                messageId: $message_id
            ) {
                discordId,
                fromDate,
                duration,   
                reason,
                approvedBy,
                appliedAt
            }
            }
        "#;

        let variables = serde_json::json!({
            "discord_id": discord_id,
            "start_date": start_date.format("%Y-%m-%d").to_string(),
            "duration": duration,
            "reason": reason,
            "message_id": message_id.to_string()
        });

        debug!("Sending query {}", query);
        debug!("With variables: {:?}", variables);

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
            .context("Failed to successfully post request")?;

        let json: Value = response
            .json()
            .await
            .context("Failed to parse response JSON")?;

        let leave_value = json["data"]["leaveApplication"].clone();

        let leave: LeaveRecord =
            serde_json::from_value(leave_value).context("Failed to deserialize LeaveRecord")?;

        Ok(leave)
    }

    pub async fn approve_leave(
        &self,
        discord_id: &str,
        from_date: NaiveDate,
        approved_by: &str,
    ) -> anyhow::Result<LeaveRecord> {
        let query = r#"
            mutation($discord_id: String!, $mentor_discord_id : String!, $from_date : String!) {
            approveLeave(
                discordId: $discord_id,
                approvedBy: $mentor_discord_id,
                fromDate: $from_date                
            ) {
                discordId,
                fromDate, 
                duration,
                reason,
                approvedBy,
                appliedAt
            }
            }
        "#;

        let variables = serde_json::json!({
            "discord_id": discord_id,
            "mentor_discord_id" : approved_by,
            "from_date": from_date.format("%Y-%m-%d").to_string()
        });

        debug!("Sending query {}", query);
        debug!("With variables: {:?}", variables);

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
            .context("Failed to successfully post request")?;

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse response JSON")?;

        if let Some(errors) = json.get("errors") {
            anyhow::bail!("GraphQL errors: {}", errors);
        }

        let leave_value = json["data"]["approveLeave"].clone();

        let leave: LeaveRecord =
            serde_json::from_value(leave_value).context("Failed to deserialize LeaveRecord")?;
        Ok(leave)
    }
}
