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
use crate::graphql::GraphQLClient;
use anyhow::Context;
use async_trait::async_trait;
use serenity::builder::CreateWebhook;
use serenity::builder::ExecuteWebhook;
use serenity::client::Context as ClientContext;
use serenity::model::id::UserId;
use serenity::model::webhook::Webhook;
use serenity::prelude::CacheHttp;
use std::collections::HashMap;

pub struct MirrorNewUpdates;

use crate::utils::time::time_until;
use chrono::{Datelike, Duration, Local, Timelike};
use chrono_tz::Asia::Kolkata;
use mailparse::{MailHeaderMap, ParsedMail};
use poise::serenity_prelude::ChannelId;

pub struct EmailDetails {
    pub from: String,
    pub body: String,
}

use crate::ids::{
    AI_STATUS_UPDATE_CHANNEL_ID, GROUP_FOUR_STATUS_UPDATE_CHANNEL_ID,
    GROUP_ONE_STATUS_UPDATE_CHANNEL_ID, GROUP_THREE_STATUS_UPDATE_CHANNEL_ID,
    GROUP_TWO_STATUS_UPDATE_CHANNEL_ID, MOBILE_STATUS_UPDATE_CHANNEL_ID, STATUS_UPDATE_CHANNEL_ID,
    SYSTEMS_STATUS_UPDATE_CHANNEL_ID, WEB_STATUS_UPDATE_CHANNEL_ID,
};

#[async_trait]
impl Task for MirrorNewUpdates {
    fn name(&self) -> &str {
        "mirror_new_updates"
    }

    fn run_in(&self) -> tokio::time::Duration {
        time_until(7, 00)
    }

    async fn run(&self, ctx: ClientContext, client: GraphQLClient) -> anyhow::Result<()> {
        mirror_new_updates(ctx, client).await
    }
}

pub async fn mirror_new_updates(ctx: ClientContext, client: GraphQLClient) -> anyhow::Result<()> {
    let emails = tokio::task::spawn_blocking(fetch_inbox).await??;
    if emails.is_empty() {
        return Ok(());
    }
    let member_data = client.fetch_member_data(Local::now().date_naive()).await?;

    let mut members = HashMap::new();
    for member in &member_data {
        members.insert(member.email.trim().to_lowercase(), member);
    }

    for email in emails {
        let sender_email = email.from.trim().to_lowercase();

        if let Some(member) = members.get(&sender_email) {
            if member.track.is_none() || member.group_id.is_none() {
                continue;
            }

            send_update(
                &ctx,
                member.name.clone(),
                member.discord_id.as_deref(),
                member.track.clone().unwrap(),
                member.group_id.unwrap(),
                email.body.clone(),
            )
            .await?;
        }
    }
    Ok(())
}

async fn get_or_create_webhook(
    ctx: &ClientContext,
    channel_id: ChannelId,
) -> anyhow::Result<Webhook> {
    let hooks = channel_id.webhooks(ctx.http()).await?;

    if let Some(hook) = hooks
        .into_iter()
        .find(|h| h.name == Some("amD Updates".to_string()))
    {
        return Ok(hook);
    }

    let webhook = channel_id
        .create_webhook(ctx.http(), CreateWebhook::new("amD Updates"))
        .await?;

    Ok(webhook)
}

async fn send_webhook_message(
    ctx: &ClientContext,
    channel_id: ChannelId,
    username: String,
    avatar_url: String,
    content: String,
) -> anyhow::Result<()> {
    let webhook = get_or_create_webhook(ctx, channel_id).await?;

    let builder = ExecuteWebhook::new()
        .username(username)
        .avatar_url(avatar_url)
        .content(content);

    webhook.execute(ctx.http(), false, builder).await?;

    Ok(())
}

async fn get_avatar_url(ctx: &ClientContext, discord_id: u64) -> anyhow::Result<String> {
    let user = ctx.http.get_user(UserId::new(discord_id)).await?;

    let avatar = user
        .avatar_url()
        .unwrap_or_else(|| user.default_avatar_url());

    Ok(avatar)
}

async fn get_bot_avatar_url(ctx: &ClientContext) -> anyhow::Result<String> {
    let bot_user = ctx.http.get_current_user().await?;

    Ok(bot_user
        .avatar_url()
        .unwrap_or_else(|| bot_user.default_avatar_url()))
}

async fn resolve_avatar_url(
    ctx: &ClientContext,
    discord_id: Option<&str>,
) -> anyhow::Result<String> {
    match discord_id {
        Some(id_str) => match id_str.parse::<u64>() {
            Ok(id) => get_avatar_url(ctx, id).await,
            Err(_) => get_bot_avatar_url(ctx).await,
        },
        None => get_bot_avatar_url(ctx).await,
    }
}

async fn send_update(
    ctx: &ClientContext,
    name: String,
    discord_id: Option<&str>,
    track: String,
    group: i32,
    content: String,
) -> anyhow::Result<()> {
    let channel_id = match (track.as_str(), group) {
        ("Inductee", 1) => GROUP_ONE_STATUS_UPDATE_CHANNEL_ID,
        ("Inductee", 2) => GROUP_TWO_STATUS_UPDATE_CHANNEL_ID,
        ("Inductee", 3) => GROUP_THREE_STATUS_UPDATE_CHANNEL_ID,
        ("Inductee", 4) => GROUP_FOUR_STATUS_UPDATE_CHANNEL_ID,
        ("AI", _) => AI_STATUS_UPDATE_CHANNEL_ID,
        ("Web", _) => WEB_STATUS_UPDATE_CHANNEL_ID,
        ("Mobile", _) => MOBILE_STATUS_UPDATE_CHANNEL_ID,
        ("Systems", _) => SYSTEMS_STATUS_UPDATE_CHANNEL_ID,
        _ => STATUS_UPDATE_CHANNEL_ID,
    };

    let channel = ChannelId::new(channel_id);

    let avatar_url = resolve_avatar_url(ctx, discord_id).await?;

    send_webhook_message(ctx, channel, name, avatar_url, content).await?;

    Ok(())
}

fn fetch_inbox() -> anyhow::Result<Vec<EmailDetails>> {
    let domain = "imap.gmail.com";
    let client = imap::ClientBuilder::new(domain, 993)
        .connect()
        .context("Failed to connect to IMAP server")?;

    let email_id = std::env::var("AMD_EMAIL_ID").context("EMAIL_ID not found in env")?;

    let app_password =
        std::env::var("AMD_APP_PASSWORD").context("APP_PASSWORD not found in the ENV")?;

    let mut session = client
        .login(email_id, app_password)
        .map_err(|e| e.0)
        .context("Failed to authenticate with email client")?;

    session.select("INBOX").context("Failed to select INBOX")?;

    let ids = session
        .search(format!("SUBJECT \"{}\"", subject()))
        .context("Couldn't find any emails with subject: {subject()}")?;

    let mut emails = Vec::new();

    for id in ids.iter() {
        let msgs = session
            .fetch(id.to_string(), "RFC822")
            .context("Failed to fetch email with id: {id.to_string()}")?;

        for msg in msgs.iter() {
            if let Some(body) = msg.body() as Option<&[u8]> {
                let parsed =
                    mailparse::parse_mail(body).context("Couldn't parse the email body")?;
                let from = parsed.headers.get_first_value("From").unwrap_or_default();

                let clean_from = if let (Some(i1), Some(i2)) = (from.find('<'), from.find('>')) {
                    from[i1 + 1..i2].to_string()
                } else {
                    from.trim().to_string()
                };

                let txt = extract_plain_text_body(&parsed)
                    .unwrap_or_else(|| String::from_utf8_lossy(body).to_string());

                emails.push(EmailDetails {
                    from: clean_from,
                    body: strip_signature(&txt),
                });
            }
        }
    }
    session.logout().context("Failed to logout")?;
    Ok(emails)
}

// This function is defined incase updates are sent every 30 minutes
fn subject() -> String {
    let now = Local::now().with_timezone(&Kolkata);

    let subject_date = if now.hour() < 7 || (now.hour() == 7 && now.minute() <= 5) {
        now - Duration::days(1)
    } else {
        now
    };

    format!(
        "Status Update [{:02}-{:02}-{:04}]",
        subject_date.day(),
        subject_date.month(),
        subject_date.year()
    )
}

fn extract_plain_text_body(parsed: &ParsedMail) -> Option<String> {
    if parsed.ctype.mimetype == "text/plain" {
        return Some(String::from_utf8_lossy(&parsed.get_body_raw().unwrap()).to_string());
    }
    for sub in &parsed.subparts {
        if let Some(t) = extract_plain_text_body(sub) {
            return Some(t);
        }
    }
    for sub in &parsed.subparts {
        if sub.ctype.mimetype == "text/html" {
            let raw = sub.get_body_raw().unwrap();
            let html = String::from_utf8_lossy(&raw);
            return Some(html2text::from_read(html.as_bytes(), usize::MAX));
        }
    }
    None
}

fn strip_signature(text: &str) -> String {
    let mut result = Vec::new();
    for line in text.lines() {
        if line.trim() == "--" || line.trim().starts_with("On ") && line.contains(" wrote:") {
            break;
        }
        result.push(line);
    }
    result.join("\n").trim().to_string()
}
