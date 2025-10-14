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
    pub discord_id: String,
    pub track: Option<String>,
    pub year: i32,
    pub status: Option<MemberStatus>,
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
