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
use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serenity::all::{
    CacheHttp, ChannelId, Context, CreateEmbed, CreateMessage, GetMessages, Message,
};
use serenity::async_trait;

use super::Task;
use crate::graphql::models::{Member, StreakWithMemberId};
use crate::graphql::queries::{fetch_members, fetch_streaks, increment_streak, reset_streak};
use crate::ids::{
    AI_CHANNEL_ID, MOBILE_CHANNEL_ID, STATUS_UPDATE_CHANNEL_ID, SYSTEMS_CHANNEL_ID, WEB_CHANNEL_ID,
};
use crate::utils::time::time_until;

/// Checks for status updates daily at 5 AM.
pub struct StatusUpdateCheck;

#[async_trait]
impl Task for StatusUpdateCheck {
    fn name(&self) -> &str {
        "Status Update Check"
    }

    fn run_in(&self) -> tokio::time::Duration {
        time_until(5, 00)
    }

    async fn run(&self, ctx: Context) -> anyhow::Result<()> {
        status_update_check(ctx).await
    }
}

type GroupedMember = HashMap<u64, Vec<Member>>;

struct ReportConfig {
    time_valid_from: DateTime<chrono_tz::Tz>,
    keywords: Vec<&'static str>,
    special_authors: Vec<&'static str>,
}

const AMAN_SHAFEEQ: &str = "767636699077410837";
const CHANDRA_MOULI: &str = "1265880467047976970";

async fn status_update_check(ctx: Context) -> anyhow::Result<()> {
    let updates = get_updates(&ctx).await?;
    let members = fetch_members().await?;

    // naughty_list -> members who did not send updates
    let (mut naughty_list, mut nice_list) = categorize_members(&members, updates);
    update_streaks_for_members(&mut naughty_list, &mut nice_list).await?;

    let embed = generate_embed(members, naughty_list).await?;
    let msg = CreateMessage::new().embed(embed);

    let status_update_channel = ChannelId::new(STATUS_UPDATE_CHANNEL_ID);
    status_update_channel.send_message(ctx.http(), msg).await?;

    Ok(())
}

async fn get_updates(ctx: &Context) -> anyhow::Result<Vec<Message>> {
    let channel_ids = get_channel_ids();
    let mut updates = Vec::new();

    let get_messages_builder = GetMessages::new().limit(100);
    for channel in channel_ids {
        let messages = channel.messages(ctx.http(), get_messages_builder).await?;
        let valid_updates = messages.into_iter().filter(is_valid_status_update);
        updates.extend(valid_updates);
    }

    Ok(updates)
}

// TODO: Replace hardcoded set with configurable list
fn get_channel_ids() -> Vec<ChannelId> {
    vec![
        ChannelId::new(SYSTEMS_CHANNEL_ID),
        ChannelId::new(MOBILE_CHANNEL_ID),
        ChannelId::new(WEB_CHANNEL_ID),
        ChannelId::new(AI_CHANNEL_ID),
    ]
}

fn is_valid_status_update(msg: &Message) -> bool {
    let report_config = get_report_config();
    let content = msg.content.to_lowercase();

    let is_within_timeframe = DateTime::<Utc>::from_timestamp(msg.timestamp.timestamp(), 0)
        .expect("Valid timestamp")
        .with_timezone(&chrono_tz::Asia::Kolkata)
        >= report_config.time_valid_from;

    let has_required_keywords = report_config
        .keywords
        .iter()
        .all(|keyword| content.contains(keyword));
    let is_special_author = report_config
        .special_authors
        .contains(&msg.author.id.to_string().as_str());
    let is_valid_content =
        has_required_keywords || (is_special_author && content.contains("regards"));

    is_within_timeframe && is_valid_content
}

// TODO: Parts of this could also be removed from code like channel_ids
fn get_report_config() -> ReportConfig {
    let now = chrono::Utc::now().with_timezone(&chrono_tz::Asia::Kolkata);
    let yesterday = now.date_naive() - chrono::Duration::days(1);
    let time_valid_from = yesterday
        .and_hms_opt(20, 0, 0)
        .expect("Valid timestamp")
        .and_local_timezone(chrono_tz::Asia::Kolkata)
        .earliest()
        .expect("Valid timezone conversion");

    ReportConfig {
        time_valid_from,
        keywords: vec!["namah shivaya", "regards"],
        special_authors: vec![AMAN_SHAFEEQ, CHANDRA_MOULI],
    }
}

fn categorize_members(
    members: &Vec<Member>,
    updates: Vec<Message>,
) -> (GroupedMember, Vec<Member>) {
    let mut nice_list = vec![];
    let mut naughty_list = HashMap::new();

    let mut sent_updates: HashSet<String> = HashSet::new();

    for message in updates.iter() {
        sent_updates.insert(message.author.id.to_string());
    }

    for member in members {
        if sent_updates.contains(&member.discord_id) {
            nice_list.push(member.clone());
        } else {
            let group = member.group_id as u64;
            naughty_list
                .entry(group)
                .or_insert_with(Vec::new)
                .push(member.clone());
        }
    }

    (naughty_list, nice_list)
}

async fn update_streaks_for_members(
    naughty_list: &mut GroupedMember,
    nice_list: &mut Vec<Member>,
) -> anyhow::Result<()> {
    for member in nice_list {
        increment_streak(member).await?;
    }

    for members in naughty_list.values_mut() {
        for member in members {
            reset_streak(member).await?;
        }
    }

    Ok(())
}

async fn generate_embed(
    members: Vec<Member>,
    naughty_list: GroupedMember,
) -> anyhow::Result<CreateEmbed> {
    let (all_time_high, all_time_high_members, current_highest, current_highest_members) =
        get_leaderboard_stats(members).await?;
    let mut description = String::new();

    description.push_str("# Leaderboard Updates\n");

    description.push_str(&format!(
        "## All-Time High Streak: {} days\n",
        all_time_high
    ));
    description.push_str(&format_members(&all_time_high_members));

    description.push_str(&format!(
        "## Current Highest Streak: {} days\n",
        current_highest
    ));
    description.push_str(&format_members(&current_highest_members));

    if !naughty_list.is_empty() {
        description.push_str("# Defaulters\n");
        description.push_str(&format_defaulters(&naughty_list));
    }

    let embed = CreateEmbed::new()
        .title("Status Update Report")
        .description(description)
        .color(serenity::all::Colour::new(0xeab308));

    Ok(embed)
}

fn format_members(members: &[Member]) -> String {
    if members.len() <= 5 {
        let list = members
            .iter()
            .map(|member| format!("- {}", member.name))
            .collect::<Vec<_>>()
            .join("\n");

        format!("{}\n", list)
    } else {
        String::from("More than five members hold this record!\n")
    }
}

fn format_defaulters(naughty_list: &GroupedMember) -> String {
    let mut description = String::new();
    for (group, missed_members) in naughty_list {
        description.push_str(&format!("## Group {}\n", group));
        for member in missed_members {
            let status = match member.streak[0].current_streak {
                0 => ":x:",
                -1 => ":x::x:",
                _ => ":headstone:",
            };
            description.push_str(&format!("- {} | {}\n", member.name, status));
        }
    }
    description.push('\n');
    description
}

async fn get_leaderboard_stats(
    members: Vec<Member>,
) -> anyhow::Result<(i32, Vec<Member>, i32, Vec<Member>)> {
    let streaks = fetch_streaks().await?;
    let member_map: HashMap<i32, &Member> = members.iter().map(|m| (m.member_id, m)).collect();

    let (all_time_high, all_time_high_members) = find_highest_streak(&streaks, &member_map, true);
    let (current_highest, current_highest_members) =
        find_highest_streak(&streaks, &member_map, false);

    Ok((
        all_time_high,
        all_time_high_members,
        current_highest,
        current_highest_members,
    ))
}

fn find_highest_streak(
    streaks: &[StreakWithMemberId],
    member_map: &HashMap<i32, &Member>,
    is_all_time: bool,
) -> (i32, Vec<Member>) {
    let mut highest = 0;
    let mut highest_members = Vec::new();

    for streak in streaks {
        if let Some(member) = member_map.get(&streak.member_id) {
            let streak_value = if is_all_time {
                streak.max_streak
            } else {
                streak.current_streak
            };

            match streak_value.cmp(&highest) {
                std::cmp::Ordering::Greater => {
                    highest = streak_value;
                    highest_members.clear();
                    highest_members.push((*member).clone());
                }
                std::cmp::Ordering::Equal => {
                    highest_members.push((*member).clone());
                }
                _ => {}
            }
        }
    }

    (highest, highest_members)
}
