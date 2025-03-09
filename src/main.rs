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
mod commands;
mod graphql;
mod ids;
mod reaction_roles;
/// This module is a simple cron equivalent. It spawns threads for the [`Task`]s that need to be completed.
mod scheduler;
/// A trait to define a job that needs to be executed regularly, for example checking for status updates daily.
mod tasks;
mod utils;

mod config;

use config::CONFIG;

use anyhow::Context as _;
use poise::{Context as PoiseContext, Framework, FrameworkOptions, PrefixFrameworkOptions};
use reaction_roles::{handle_reaction, populate_data_with_reaction_roles};
use serenity::{
    all::{ReactionType, RoleId, UserId},
    client::{Context as SerenityContext, FullEvent},
    model::gateway::GatewayIntents,
};
use tokio::sync::RwLock;
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, reload, EnvFilter, Registry};

use std::{
    collections::{HashMap, HashSet},
    fs::File,
    sync::Arc,
};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = PoiseContext<'a, Data, Error>;
pub type ReloadHandle = Arc<RwLock<reload::Handle<EnvFilter, Registry>>>;

pub struct Data {
    pub reaction_roles: HashMap<ReactionType, RoleId>,
    pub log_reload_handle: ReloadHandle,
}

fn setup_tracing() -> anyhow::Result<ReloadHandle> {
    let env = std::env::var("AMD_RUST_ENV").context("RUST_ENV was not found in the ENV")?;
    let enable_debug_libraries_string = std::env::var("ENABLE_DEBUG_LIBRARIES")
        .context("ENABLE_DEBUG_LIBRARIES was not found in the ENV")?;
    let enable_debug_libraries: bool = enable_debug_libraries_string
        .parse()
        .context("Failed to parse ENABLE_DEBUG_LIBRARIES")?;
    let crate_name = env!("CARGO_CRATE_NAME");

    let (filter, reload_handle) = reload::Layer::new(EnvFilter::new(
        if env == "production" && enable_debug_libraries {
            "info".to_string()
        } else if env == "production" && !enable_debug_libraries {
            format!("{crate_name}=info")
        } else if enable_debug_libraries {
            "trace".to_string()
        } else {
            format!("{crate_name}=trace")
        },
    ));

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
    let reload_handle = setup_tracing().context("Failed to setup tracing")?;

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
