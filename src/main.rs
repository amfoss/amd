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
/// Contains all the commands for the bot.
mod commands;
/// Interact with [Root's](https://www.github.com/amfoss/root) GraphQL interace.
mod graphql;
/// Contains Discord IDs that may be needed across the bot.
mod ids;
/// This module is a simple cron equivalent. It spawns threads for the [`Task`]s that need to be completed.
mod scheduler;
/// A trait to define a job that needs to be executed regularly, for example checking for status updates daily.
mod tasks;
/// Misc. helper functions that don't really have a place anywhere else.
mod utils;

use anyhow::Context as _;
use poise::{Context as PoiseContext, Framework, FrameworkOptions, PrefixFrameworkOptions};
use serenity::{
    all::{Reaction, ReactionType, RoleId, UserId},
    client::{Context as SerenityContext, FullEvent},
    model::{gateway::GatewayIntents, id::MessageId},
};
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, reload, EnvFilter, Registry};

use std::{
    collections::{HashMap, HashSet},
    fs::File,
    sync::Arc,
};

use ids::{
    AI_ROLE_ID, ARCHIVE_ROLE_ID, DEVOPS_ROLE_ID, MOBILE_ROLE_ID, RESEARCH_ROLE_ID,
    ROLES_MESSAGE_ID, SYSTEMS_ROLE_ID, WEB_ROLE_ID,
};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = PoiseContext<'a, Data, Error>;
pub type ReloadHandle = Arc<RwLock<reload::Handle<EnvFilter, Registry>>>;

pub struct Data {
    pub reaction_roles: HashMap<ReactionType, RoleId>,
    pub log_reload_handle: ReloadHandle,
}

/// This function is responsible for adding all the (emoji, role_id) pairs used in the
/// `event_handler` to Data
pub fn populate_data_with_reaction_roles(data: &mut Data) {
    let roles = [
        (
            ReactionType::Unicode("📁".to_string()),
            RoleId::new(ARCHIVE_ROLE_ID),
        ),
        (
            ReactionType::Unicode("📱".to_string()),
            RoleId::new(MOBILE_ROLE_ID),
        ),
        (
            ReactionType::Unicode("⚙️".to_string()),
            RoleId::new(SYSTEMS_ROLE_ID),
        ),
        (
            ReactionType::Unicode("🤖".to_string()),
            RoleId::new(AI_ROLE_ID),
        ),
        (
            ReactionType::Unicode("📜".to_string()),
            RoleId::new(RESEARCH_ROLE_ID),
        ),
        (
            ReactionType::Unicode("🚀".to_string()),
            RoleId::new(DEVOPS_ROLE_ID),
        ),
        (
            ReactionType::Unicode("🌐".to_string()),
            RoleId::new(WEB_ROLE_ID),
        ),
    ];

    data.reaction_roles
        .extend::<HashMap<ReactionType, RoleId>>(roles.into());
}

/// Abstraction over initializing the global subscriber for tracing depending on whether it's in production or dev.
fn setup_tracing(env: &str, enable_libraries: bool) -> anyhow::Result<ReloadHandle> {
    let crate_name = env!("CARGO_CRATE_NAME");
    let (filter, reload_handle) =
        reload::Layer::new(EnvFilter::new(if env == "production" && enable_libraries {
            "info".to_string()
        } else if env == "production" && !enable_libraries {
            format!("{crate_name}=info")
        } else if enable_libraries {
            "trace".to_string()
        } else {
            format!("{crate_name}=trace")
        }));

    if env != "production" {
        let subscriber = tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().pretty().with_writer(std::io::stdout))
            .with(
                fmt::layer()
                    .pretty()
                    .with_ansi(false)
                    .with_writer(File::create("amd.log").context("Failed to create subscriber")?),
            );

        tracing::subscriber::set_global_default(subscriber).context("Failed to set subscriber")?;
        Ok(Arc::new(RwLock::new(reload_handle)))
    } else {
        let subscriber = tracing_subscriber::registry().with(filter).with(
            fmt::layer()
                .pretty()
                .with_ansi(false)
                .with_writer(File::create("amd.log").context("Failed to create subscriber")?),
        );

        tracing::subscriber::set_global_default(subscriber).context("Failed to set subscriber")?;
        Ok(Arc::new(RwLock::new(reload_handle)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();
    let is_production =
        std::env::var("AMD_RUST_ENV").context("RUST_ENV was not found in the ENV")?;
    let enable_debug_libraries_string = std::env::var("ENABLE_DEBUG_LIBRARIES")
        .context("ENABLE_DEBUG_LIBRARIES was not found in the ENV")?;
    let enable_debug_libraries: bool = enable_debug_libraries_string
        .parse()
        .context("Failed to parse ENABLE_DEBUG_LIBRARIES")?;
    let reload_handle =
        setup_tracing(&is_production, enable_debug_libraries).context("Failed to setup tracing")?;

    info!("Tracing initialized. Continuing main...");
    let mut data = Data {
        reaction_roles: HashMap::new(),
        log_reload_handle: reload_handle,
    };
    populate_data_with_reaction_roles(&mut data);

    let discord_token =
        std::env::var("DISCORD_TOKEN").context("DISCORD_TOKEN was not found in the ENV")?;
    let owner_id: u64 = std::env::var("OWNER_ID")
        .context("OWNER_ID was not found in the ENV")?
        .parse()
        .context("Failed to parse owner_id")?;
    let owner_user_id = UserId::from(owner_id);

    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: commands::get_commands(),
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            prefix_options: PrefixFrameworkOptions {
                prefix: Some(String::from("$")),
                ..Default::default()
            },
            owners: HashSet::from([owner_user_id]),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                scheduler::run_scheduler(ctx.clone()).await;
                Ok(data)
            })
        })
        .build();

    let mut client = serenity::client::ClientBuilder::new(
        discord_token,
        GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT,
    )
    .framework(framework)
    .await
    .context("Failed to create the Serenity client")?;

    client
        .start()
        .await
        .context("Failed to start the Serenity client")?;

    info!("Starting amD...");

    Ok(())
}

/// Handles various events from Discord, such as reactions.
async fn event_handler(
    ctx: &SerenityContext,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        FullEvent::ReactionAdd { add_reaction } => {
            handle_reaction(ctx, add_reaction, data, true).await;
        }
        FullEvent::ReactionRemove { removed_reaction } => {
            handle_reaction(ctx, removed_reaction, data, false).await;
        }
        _ => {}
    }

    Ok(())
}

/// Handles adding or removing roles based on reactions.
async fn handle_reaction(ctx: &SerenityContext, reaction: &Reaction, data: &Data, is_add: bool) {
    if !is_relevant_reaction(reaction.message_id, &reaction.emoji, data) {
        return;
    }

    debug!("Handling {:?} from {:?}.", reaction.emoji, reaction.user_id);

    // TODO Log these errors
    let Some(guild_id) = reaction.guild_id else {
        return;
    };
    let Some(user_id) = reaction.user_id else {
        return;
    };
    let Ok(member) = guild_id.member(ctx, user_id).await else {
        return;
    };
    let Some(role_id) = data.reaction_roles.get(&reaction.emoji) else {
        return;
    };

    let result = if is_add {
        member.add_role(&ctx.http, *role_id).await
    } else {
        member.remove_role(&ctx.http, *role_id).await
    };

    if let Err(e) = result {
        error!(
            "Could not handle {:?} from {:?}. Error: {}",
            reaction.emoji, reaction.user_id, e
        );
    }
}

/// Helper function to check if a reaction was made to [`ids::ROLES_MESSAGE_ID`] and if [`Data::reaction_roles`] contains a relevant (emoji, role) pair.
fn is_relevant_reaction(message_id: MessageId, emoji: &ReactionType, data: &Data) -> bool {
    message_id == MessageId::new(ROLES_MESSAGE_ID) && data.reaction_roles.contains_key(emoji)
}
