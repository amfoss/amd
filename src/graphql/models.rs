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
use chrono::{NaiveDate, NaiveDateTime};
use serde::Deserialize;
#[derive(Clone, Debug, Deserialize)]
pub struct StatusOnDate {
    #[serde(rename = "isSent")]
    pub is_sent: bool,
    #[serde(rename = "onBreak")]
    pub on_break: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct StatusStreak {
    #[serde(rename = "currentStreak")]
    pub current_streak: Option<i32>,
    #[serde(rename = "maxStreak")]
    pub max_streak: Option<i32>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MemberStatus {
    #[serde(rename = "onDate")]
    pub on_date: Option<StatusOnDate>,
    pub streak: Option<StatusStreak>,
    #[serde(rename = "consecutiveMisses")]
    pub consecutive_misses: Option<i32>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Member {
    #[serde(rename = "memberId")]
    pub member_id: i32,
    pub name: String,
    #[serde(rename = "discordId")]
    pub discord_id: Option<String>,
    pub track: Option<String>,
    pub year: Option<i32>,
    pub status: Option<MemberStatus>,
    #[serde(rename = "groupId")]
    pub group_id: Option<i32>,
    pub email: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AttendanceRecord {
    pub name: String,
    pub year: i32,
    #[serde(rename = "isPresent")]
    pub is_present: bool,
    #[serde(rename = "timeIn")]
    pub time_in: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MemberSummary {
    #[serde(rename = "presentPercent")]
    pub present_percent: f32,
    #[serde(rename = "updatesPercent")]
    pub updates_percent: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LeaveCountRecord {
    #[serde(rename = "discordId")]
    pub discord_id: String,
    #[serde(rename = "leaveCount")]
    pub leave_count: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LeaveRecord {
    #[serde(rename = "discordId")]
    pub discord_id: String,
    #[serde(rename = "fromDate")]
    pub from_date: NaiveDate,
    pub duration: i32,
    pub reason: Option<String>,
    #[serde(rename = "approvedBy")]
    pub approved_by: Option<String>,
    #[serde(rename = "appliedAt")]
    pub applied_at: NaiveDateTime,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LeaveRecordWithMessage {
    #[serde(flatten)]
    pub leave: LeaveRecord,
    #[serde(rename = "messageId")]
    pub message_id: String,
}
