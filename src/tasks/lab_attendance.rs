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
use super::Task;
use anyhow::Context as _;
use chrono::{DateTime, Datelike, Local, NaiveTime, ParseError, TimeZone, Timelike, Utc};
use serenity::all::{
    ChannelId, Colour, Context as SerenityContext, CreateEmbed, CreateEmbedAuthor, CreateMessage,
};
use serenity::async_trait;
use std::collections::HashMap;
use tracing::{debug, trace};

use crate::{
    graphql::{models::AttendanceRecord, queries::fetch_attendance},
    ids::THE_LAB_CHANNEL_ID,
    utils::time::{get_five_forty_five_pm_timestamp, time_until},
};

const TITLE_URL: &str = "https://www.amfoss.in/";
const AUTHOR_URL: &str = "https://github.com/amfoss/amd";

pub struct PresenseReport;

#[async_trait]
impl Task for PresenseReport {
    fn name(&self) -> &str {
        "Lab Attendance Check"
    }

    fn run_in(&self) -> tokio::time::Duration {
        time_until(18, 00)
    }

    async fn run(&self, ctx: SerenityContext) -> anyhow::Result<()> {
        check_lab_attendance(ctx).await
    }
}

pub async fn check_lab_attendance(ctx: SerenityContext) -> anyhow::Result<()> {
    trace!("Starting lab attendance check");
    let attendance = fetch_attendance()
        .await
        .context("Failed to fetch attendance from Root")?;

    let time = Local::now().with_timezone(&chrono_tz::Asia::Kolkata);
    let threshold_time = get_five_forty_five_pm_timestamp(time);

    let mut absent_list = Vec::new();
    let mut late_list = Vec::new();

    for record in &attendance {
        debug!("Checking attendance for member: {}", record.name);
        if !record.is_present || record.time_in.is_none() {
            absent_list.push(record.clone());
            debug!("Member {} marked as absent", record.name);
        } else if let Some(time_str) = &record.time_in {
            if let Ok(time) = parse_time(time_str) {
                if time > threshold_time {
                    late_list.push(record.clone());
                    debug!("Member {} marked as late", record.name);
                }
            }
        }
    }

    if absent_list.len() == attendance.len() {
        send_lab_closed_message(ctx).await?;
    } else {
        send_attendance_report(ctx, absent_list, late_list, attendance.len()).await?;
    }

    trace!("Completed lab attendance check");
    Ok(())
}

async fn send_lab_closed_message(ctx: SerenityContext) -> anyhow::Result<()> {
    let today_date = Utc::now().format("%B %d, %Y").to_string();

    let bot_user = ctx.http.get_current_user().await?;
    let bot_avatar_url = bot_user
        .avatar_url()
        .unwrap_or_else(|| bot_user.default_avatar_url());

    let embed = CreateEmbed::new()
        .title(format!("Presense Report - {}", today_date))
        .url(TITLE_URL)
        .author(
            CreateEmbedAuthor::new("amD")
                .url(AUTHOR_URL)
                .icon_url(bot_avatar_url),
        )
        .color(Colour::RED)
        .description("Uh-oh, seems like the lab is closed today! 🏖️ Everyone is absent!")
        .timestamp(Utc::now());

    ChannelId::new(THE_LAB_CHANNEL_ID)
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await
        .context("Failed to send lab closed message")?;

    Ok(())
}

async fn send_attendance_report(
    ctx: SerenityContext,
    absent_list: Vec<AttendanceRecord>,
    late_list: Vec<AttendanceRecord>,
    total_count: usize,
) -> anyhow::Result<()> {
    let today_date = Utc::now().format("%B %d, %Y").to_string();

    let present = total_count - absent_list.len();
    let attendance_percentage = if total_count > 0 {
        (present as f32 / total_count as f32) * 100.0
    } else {
        0.0
    };

    let bot_user = ctx.http.get_current_user().await?;
    let bot_avatar_url = bot_user
        .avatar_url()
        .unwrap_or_else(|| bot_user.default_avatar_url());

    let embed_color = if attendance_percentage > 75.0 {
        Colour::DARK_GREEN
    } else if attendance_percentage > 50.0 {
        Colour::GOLD
    } else {
        Colour::RED
    };

    let mut description = format!(
        "# Stats\n- Present: {} ({}%)\n- Absent: {}\n- Late: {}\n\n",
        present,
        attendance_percentage.round() as i32,
        absent_list.len(),
        late_list.len()
    );

    description.push_str(&format_attendance_list("Absent", &absent_list));
    description.push_str(&format_attendance_list("Late", &late_list));

    let embed = CreateEmbed::new()
        .title(format!("Presense Report - {}", today_date))
        .url(TITLE_URL)
        .author(
            CreateEmbedAuthor::new("amD")
                .url(AUTHOR_URL)
                .icon_url(bot_avatar_url),
        )
        .color(embed_color)
        .description(description)
        .timestamp(Utc::now());

    ChannelId::new(THE_LAB_CHANNEL_ID)
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await
        .context("Failed to send attendance report")?;

    Ok(())
}

fn format_attendance_list(title: &str, list: &[AttendanceRecord]) -> String {
    if list.is_empty() {
        return format!(
            "**{}**\nNo one is {} today! 🎉\n\n",
            title,
            title.to_lowercase()
        );
    }

    let mut by_year: HashMap<i32, Vec<&str>> = HashMap::new();
    for record in list {
        if record.year >= 1 && record.year <= 3 {
            by_year.entry(record.year).or_default().push(&record.name);
        }
    }

    let mut result = format!("# {}\n", title);

    for year in 1..=3 {
        if let Some(names) = by_year.get(&year) {
            if !names.is_empty() {
                result.push_str(&format!("### Year {}\n", year));

                for name in names {
                    result.push_str(&format!("- {}\n", name));
                }
                result.push('\n');
            }
        }
    }

    result
}

fn parse_time(time_str: &str) -> Result<DateTime<Local>, ParseError> {
    let time_only = time_str.split('.').next().unwrap();
    let naive_time = NaiveTime::parse_from_str(time_only, "%H:%M:%S")?;
    let now = Local::now();

    let result = Local
        .with_ymd_and_hms(
            now.year(),
            now.month(),
            now.day(),
            naive_time.hour(),
            naive_time.minute(),
            naive_time.second(),
        )
        .single()
        .expect("Valid datetime must be created");

    Ok(result)
}
