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

use serenity::all::{CacheHttp, ChannelId, Context, CreateEmbed, CreateMessage, GuildId};
use serenity::async_trait;
use tracing::instrument;
use tracing::{info, warn};

use super::Task;
use crate::graphql::models::Member;
use crate::graphql::GraphQLClient;
use crate::ids::{AMFOSS_GUILD_ID, STATUS_UPDATE_CHANNEL_ID};
use crate::utils::time::time_until;

/// Checks for status updates daily at 5 AM.
pub struct StatusUpdateReport;

#[async_trait]
impl Task for StatusUpdateReport {
    fn name(&self) -> &str {
        "Status Update Report"
    }

    fn run_in(&self) -> tokio::time::Duration {
        time_until(6, 45)
        // Duration::from_secs(1) // for development
    }

    async fn run(&self, ctx: Context, client: GraphQLClient) -> anyhow::Result<()> {
        status_update_check(ctx, client).await
    }
}

type GroupedMember = HashMap<Option<String>, Vec<Member>>;

#[instrument(level = "debug", skip(ctx))]
pub async fn status_update_check(ctx: Context, client: GraphQLClient) -> anyhow::Result<()> {
    let now = chrono::Utc::now().with_timezone(&chrono_tz::Asia::Kolkata);
    let yesterday = now.date_naive() - chrono::Duration::days(1);

    let mut members = client.fetch_member_data(yesterday).await?;
    members.retain(|member| member.year != Some(4));

    // naughty_list -> members who did not send updates
    let (naughty_list, years_on_break) = categorize_members(&members);

    kick_lazy_bums(&ctx, naughty_list.values().flatten().cloned().collect()).await;
    let embed = generate_embed(members, naughty_list, years_on_break).await?;
    let msg = CreateMessage::new().embed(embed);

    let status_update_channel = ChannelId::new(STATUS_UPDATE_CHANNEL_ID);
    status_update_channel.send_message(ctx.http(), msg).await?;

    Ok(())
}

fn categorize_members(members: &Vec<Member>) -> (GroupedMember, Vec<i32>) {
    let mut naughty_list: HashMap<Option<String>, Vec<Member>> = HashMap::new();
    let mut years_on_break: HashSet<i32> = HashSet::new();

    for member in members {
        let Some(status) = &member.status else {
            continue;
        };
        let Some(on_date) = &status.on_date else {
            continue;
        };

        if on_date.on_break {
            if let Some(year) = member.year {
                years_on_break.insert(year);
            }
            continue;
        }

        if !on_date.is_sent {
            let track = member.track.clone();
            naughty_list.entry(track).or_default().push(member.clone());
        }
    }

    (naughty_list, years_on_break.into_iter().collect())
}

async fn generate_embed(
    members: Vec<Member>,
    naughty_list: GroupedMember,
    years_on_break: Vec<i32>,
) -> anyhow::Result<CreateEmbed> {
    let (all_time_high, all_time_high_members, current_highest, current_highest_members) =
        get_leaderboard_stats(members).await?;
    let mut description = String::new();

    description.push_str("# Leaderboard Updates\n");

    description.push_str(&format!("## All-Time High Streak: {all_time_high} days\n"));
    description.push_str(&format_members(&all_time_high_members));

    description.push_str(&format!(
        "## Current Highest Streak: {current_highest} days\n"
    ));
    description.push_str(&format_members(&current_highest_members));

    if !years_on_break.is_empty() {
        description.push_str("## The Following Batches Are On Break:\n");
        description.push_str(&format_breaks(years_on_break));
    }

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

        format!("{list}\n")
    } else {
        String::from("More than five members hold this record!\n")
    }
}

fn format_breaks(mut years_on_break: Vec<i32>) -> String {
    years_on_break.sort();
    let list = years_on_break
        .iter()
        .map(|&year| {
            let year_label = match year {
                1 => "First Years",
                2 => "Second Years",
                3 => "Third Years",
                _ => return format!("Year {}", year),
            };
            format!("- {}", year_label)
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("{list}\n")
}

async fn kick_lazy_bums(ctx: &Context, naughty_list: Vec<Member>) {
    let guild_id = GuildId::new(AMFOSS_GUILD_ID);

    for member in naughty_list {
        let consecutive_misses = member
            .status
            .as_ref()
            .and_then(|s| s.consecutive_misses)
            .unwrap_or(0);

        if consecutive_misses > 3 {
            let Some(id_str) = member.discord_id.as_deref() else {
                warn!("Cannot kick {}: Missing Discord ID", member.name);
                return;
            };

            let discord_id: u64 = match id_str.parse() {
                Ok(id) => id,
                Err(_) => {
                    warn!(
                        "Cannot kick {}: Invalid Discord ID '{}'",
                        member.name, id_str
                    );
                    return;
                }
            };
            let reason = "You have been kicked for not sending status updates, reach out to a mentor for further details.";

            match guild_id
                .kick_with_reason(ctx.http(), discord_id, reason)
                .await
            {
                Ok(_) => {
                    info!(
                        "Kicked Member: {}, ID: {} for failing to send updates.",
                        member.name, member.member_id
                    )
                }
                Err(e) => {
                    info!(
                        "Failed to Kick (Name: {}, ID: {}): {}",
                        member.name, member.member_id, e
                    )
                }
            }
        }
    }
}

fn format_defaulters(naughty_list: &GroupedMember) -> String {
    let mut description = String::new();
    for (track, missed_members) in naughty_list {
        match track {
            Some(t) => description.push_str(&format!("## Track - {t}\n")),
            None => description.push_str("## Unassigned\n"),
        }

        for member in missed_members {
            let status = match member.status.as_ref().and_then(|s| s.consecutive_misses) {
                None => ":zzz:",
                Some(1) => ":x:",
                Some(2) => ":x::x:",
                Some(3) => ":x::x::x:",
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
    let (all_time_high, all_time_high_members) = find_highest_streak(&members, true);
    let (current_highest, current_highest_members) = find_highest_streak(&members, false);

    Ok((
        all_time_high,
        all_time_high_members,
        current_highest,
        current_highest_members,
    ))
}

fn find_highest_streak(members: &Vec<Member>, is_all_time: bool) -> (i32, Vec<Member>) {
    let mut highest = 0;
    let mut highest_members = Vec::new();

    for member in members {
        let streak_value = member
            .status
            .as_ref()
            .and_then(|s| s.streak.as_ref())
            .and_then(|streak| {
                if is_all_time {
                    streak.max_streak
                } else {
                    streak.current_streak
                }
            })
            .unwrap_or(0); // default to 0 if no streak info

        match streak_value.cmp(&highest) {
            std::cmp::Ordering::Greater => {
                highest = streak_value;
                highest_members.clear();
                highest_members.push(member.clone());
            }
            std::cmp::Ordering::Equal => {
                highest_members.push(member.clone());
            }
            _ => {}
        }
    }

    (highest, highest_members)
}
