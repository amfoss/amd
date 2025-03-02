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
use anyhow::{anyhow, Context as _};
use chrono::{Datelike, TimeZone};
use chrono_tz::Asia;
use serenity::all::{
    ChannelId, Context, CreateEmbed, CreateEmbedAuthor, CreateMessage, Message, MessageId,
    Timestamp,
};
use serenity::async_trait;
use tracing::{debug, trace};

use std::io::Write;
use std::{collections::HashSet, fs::File};

use super::Task;
use crate::utils::time::time_until;
use crate::{
    graphql::{
        models::Member,
        queries::{fetch_members, increment_streak, reset_streak},
    },
    ids::{
        GROUP_FOUR_CHANNEL_ID, GROUP_ONE_CHANNEL_ID, GROUP_THREE_CHANNEL_ID, GROUP_TWO_CHANNEL_ID,
        STATUS_UPDATE_CHANNEL_ID,
    },
};

const TITLE_URL: &str = "https://www.youtube.com/watch?v=epnuvyNj0FM";
const IMAGE_URL: &str = "https://media1.tenor.com/m/zAHCPvoyjNIAAAAd/yay-kitty.gif";
const AUTHOR_URL: &str = "https://github.com/amfoss/amd";
const ICON_URL: &str = "https://cdn.discordapp.com/avatars/1245352445736128696/da3c6f833b688f5afa875c9df5d86f91.webp?size=160";

/// Checks for status updates daily at 9 AM.
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
        check_status_updates(ctx).await
    }
}

pub async fn check_status_updates(ctx: Context) -> anyhow::Result<()> {
    trace!("Starting check_status_updates");
    let members = fetch_members()
        .await
        .context("Failed to fetch members from Root.")?;
    debug!("Members fetched from root: {:?}", members);
    let channel_ids = get_channel_ids().context("Failed to get channel IDs")?;
    debug!("channel_ids: {:?}", channel_ids);
    let updates: Vec<Message> = collect_updates(&channel_ids, &ctx)
        .await
        .context("Failed to collect updates")?;
    debug!("Updates collected: {:?}", updates);
    send_and_save_limiting_messages(&channel_ids, &ctx)
        .await
        .context("Failed to send and save limiting messages")?;
    let embed = generate_embed(members, updates)
        .await
        .context("Failed to generate embed")?;
    let msg = CreateMessage::new().embed(embed);
    let status_update_channel = ChannelId::new(STATUS_UPDATE_CHANNEL_ID);
    debug!("Sending report...");
    status_update_channel
        .send_message(ctx.http, msg)
        .await
        .context("Failed to send status update report")?;

    Ok(())
}

// TOOD: Get IDs through ENV instead
fn get_channel_ids() -> anyhow::Result<Vec<ChannelId>> {
    trace!("Getting channel ids...");
    Ok(vec![
        ChannelId::new(GROUP_ONE_CHANNEL_ID),
        ChannelId::new(GROUP_TWO_CHANNEL_ID),
        ChannelId::new(GROUP_THREE_CHANNEL_ID),
        ChannelId::new(GROUP_FOUR_CHANNEL_ID),
    ])
}

async fn send_and_save_limiting_messages(
    channel_ids: &Vec<ChannelId>,
    ctx: &Context,
) -> anyhow::Result<()> {
    trace!("Running send_and_save_limiting_messages()");
    let mut msg_ids: Vec<MessageId> = vec![];
    for channel_id in channel_ids {
        debug!("Sending message in {}", channel_id);
        let msg = channel_id
            .say(
                &ctx.http,
                "Collecting messages for status update report. Please do not delete this message.",
            )
            .await
            .with_context(|| {
                anyhow::anyhow!("Failed to send limiting message in channel {}", channel_id)
            })?;

        debug!("Message ID: {}", msg.id);
        msg_ids.push(msg.id);
    }
    let file_name =
        std::env::var("CONFIG_FILE_NAME").context("Config. file name was not found in the ENV")?;
    let mut file = File::create(file_name).context("Failed to create Config. file handler")?;

    for msg_id in msg_ids {
        writeln!(file, "{}", msg_id).context("Failed to write to Config. file")?;
    }

    Ok(())
}

async fn collect_updates(channel_ids: &[ChannelId], ctx: &Context) -> anyhow::Result<Vec<Message>> {
    trace!("Collecting updates");
    let mut valid_updates: Vec<Message> = vec![];
    let message_ids = get_msg_ids();
    match message_ids {
        Ok(message_ids) => {
            let now = chrono::Local::now().with_timezone(&chrono_tz::Asia::Kolkata);
            let today_five_am = chrono::Local
                .with_ymd_and_hms(now.year(), now.month(), now.day(), 5, 0, 0)
                .earliest()
                .expect("Failed to create 5 AM timestamp");
            let yesterday_five_pm = today_five_am - chrono::Duration::hours(12);
            for (&channel_id, &msg_id) in channel_ids.iter().zip(message_ids.iter()) {
                let messages = channel_id
                    .messages(
                        &ctx.http,
                        serenity::builder::GetMessages::new()
                            .after(msg_id)
                            .limit(100),
                    )
                    .await
                    .with_context(|| {
                        anyhow!("Failed to get messages from channel {}", channel_id)
                    })?;

                debug!("Messages: {:?}", messages);
                valid_updates.extend(messages.into_iter().filter(|msg| {
                    let content = msg.content.to_lowercase();
                    (content.contains("namah shivaya")
                        && content.contains("regards")
                        && msg.timestamp >= yesterday_five_pm.into())
                        || (content.contains("regards")
                            && msg.author.name == "amanoslean"
                            && msg.timestamp >= yesterday_five_pm.into())
                }));
            }

            debug!("Valid updates: {:?}", valid_updates);
            Ok(valid_updates)
        }
        Err(e) => {
            debug!(
                "Failed to get message_ids {}. Defaulting to default GetMessages()",
                e
            );
            let now = chrono::Local::now().with_timezone(&chrono_tz::Asia::Kolkata);
            let today_five_am = chrono::Local
                .with_ymd_and_hms(now.year(), now.month(), now.day(), 5, 0, 0)
                .earliest()
                .expect("Failed to create 5 AM timestamp");
            let yesterday_five_pm = today_five_am - chrono::Duration::hours(12);
            for &channel_id in channel_ids {
                let messages = channel_id
                    .messages(&ctx.http, serenity::builder::GetMessages::new().limit(100))
                    .await
                    .with_context(|| {
                        anyhow!("Failed to get messages from channel {}", channel_id)
                    })?;

                debug!("Messages: {:?}", messages);
                valid_updates.extend(messages.into_iter().filter(|msg| {
                    let content = msg.content.to_lowercase();
                    (content.contains("namah shivaya")
                        && content.contains("regards")
                        && msg.timestamp >= yesterday_five_pm.into())
                        || (content.contains("regards")
                            && msg.author.name == "amanoslean"
                            && msg.timestamp >= yesterday_five_pm.into())
                }));
            }

            debug!("Valid updates: {:?}", valid_updates);
            Ok(valid_updates)
        }
    }
}

fn get_msg_ids() -> anyhow::Result<Vec<MessageId>> {
    let file_name =
        std::env::var("CONFIG_FILE_NAME").context("Configuration file name must be present")?;
    let content =
        std::fs::read_to_string(file_name).context("Failed to read config. file for msg ids")?;

    let msg_ids = content
        .lines()
        .filter_map(|line| line.parse::<u64>().ok())
        .map(|e| MessageId::new(e))
        .collect();

    debug!("msg_ids: {:?}", msg_ids);
    Ok(msg_ids)
}

async fn generate_embed(
    members: Vec<Member>,
    messages: Vec<Message>,
) -> anyhow::Result<CreateEmbed> {
    trace!("Running generate_embed");
    let mut naughty_list: Vec<Member> = Vec::new();
    let mut highest_streak = 0;
    let mut all_time_high = 0;
    let mut all_time_high_members: Vec<Member> = Vec::new();
    let mut highest_streak_members: Vec<Member> = Vec::new();
    let mut record_breakers: Vec<Member> = vec![];

    let message_authors: HashSet<String> =
        messages.iter().map(|m| m.author.id.to_string()).collect();
    debug!("Message authors: {:?}", message_authors);

    for mut member in members.into_iter().filter(|m| m.name != "Pakhi Banchalia") {
        debug!("Processing member: {:?}", member);
        let has_sent_update = message_authors.contains(&member.discord_id);

        if has_sent_update {
            increment_streak(&mut member)
                .await
                .context("Failed to increment streak")?;
            let current_streak = member.streak[0].current_streak;
            let max_streak = member.streak[0].max_streak;

            if current_streak < 1 {
                increment_streak(&mut member)
                debug!("Setting streak to 1 for {}", member.name);
            }

            if current_streak >= highest_streak {
                debug!("Pushing to highest_streak: {:?}", member);
                highest_streak = current_streak;
                highest_streak_members.push(member.clone());
            }

            if current_streak == max_streak {
                debug!("Pushing to record_breakers: {:?}", member);
                record_breakers.push(member.clone())
            }

            if max_streak >= all_time_high {
                debug!("Pushing to all_time_high_members: {:?}", member);
                all_time_high = max_streak;
                all_time_high_members.push(member.clone())
            }
        } else {
            debug!("Pushing to naughty_list: {:?}", member);
            reset_streak(&mut member)
                .await
                .context("Failed to reset streak")?;
            naughty_list.push(member.clone());
        }
    }

    let description = build_description(
        highest_streak,
        all_time_high,
        &highest_streak_members,
        &all_time_high_members,
        &record_breakers,
        &naughty_list,
    );
    let today = chrono::Local::now()
        .with_timezone(&Asia::Kolkata)
        .date_naive();

    let mut embed = CreateEmbed::default()
        .title(format!("Status Update Report - {}", today))
        .url(TITLE_URL)
        .description(description)
        .color(serenity::all::Colour::new(0xeab308))
        .timestamp(Timestamp::now())
        .author(
            CreateEmbedAuthor::new("amD")
                .url(AUTHOR_URL)
                .icon_url(ICON_URL),
        );

    if naughty_list.is_empty() {
        embed = embed.image(IMAGE_URL);
    }

    Ok(embed)
}

fn build_description(
    highest_streak: i32,
    all_time_high: i32,
    highest_streak_members: &[Member],
    all_time_high_members: &[Member],
    record_breakers: &[Member],
    naughty_list: &[Member],
) -> String {
    trace!("Running build_description");
    let mut desc = String::from("# Leaderboard Updates\n");

    desc.push_str(&format_section(
        "All Time High",
        all_time_high,
        all_time_high_members,
    ));
    desc.push_str(&format_section(
        "Current Highest Streak",
        highest_streak,
        highest_streak_members,
    ));

    if !record_breakers.is_empty() {
        desc.push_str("## New Personal Records\n");
        for member in record_breakers {
            desc.push_str(&format!(
                "- {} - {}\n",
                member.name, member.streak[0].current_streak
            ));
        }
    }

    if naughty_list.is_empty() {
        desc.push_str("# Missed Updates\nEveryone sent their update yesterday!\n");
    } else {
        desc.push_str("# Missed Updates\n");
        for member in naughty_list {
            let status = match member.streak[0].current_streak {
                0 => ":x:",
                -1 => ":x::x:",
                _ => ":headstone:",
            };
            desc.push_str(&format!("- {} | {}\n", member.name, status));
        }
    }

    debug!("Description: {}", desc);
    desc
}

fn format_section(title: &str, value: i32, members: &[Member]) -> String {
    if members.len() > 5 {
        format!(
            "## {} - {}\nMore than five members hold the record!\n",
            title, value
        )
    } else {
        let mut section = format!("## {} - {}\n", title, value);
        for member in members {
            section.push_str(&format!("- {}\n", member.name));
        }
        section
    }
}
