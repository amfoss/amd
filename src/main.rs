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
mod config;
mod graphql;
mod ids;
mod reaction_roles;
mod scheduler;
mod tasks;
mod trace;
mod utils;

use anyhow::Context as _;
use config::Config;
use graphql::GraphQLClient;
use poise::{Context as PoiseContext, Framework, FrameworkOptions, PrefixFrameworkOptions};
use reaction_roles::handle_reaction;
use serenity::client::ClientBuilder;
use serenity::Client;
use serenity::{
    all::{ReactionType, RoleId, UserId},
    client::{Context as SerenityContext, FullEvent},
    model::gateway::GatewayIntents,
};
use trace::{setup_tracing, ReloadHandle};
use tracing::{debug, info, instrument};

use std::collections::HashMap;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = PoiseContext<'a, Data, Error>;

/// The [`Data`] struct is kept in-memory by the Bot till it shutsdown and can be used to store session-persistent data.
#[derive(Clone)]
struct Data {
    reaction_roles: HashMap<ReactionType, RoleId>,
    log_reload_handle: ReloadHandle,
    graphql_client: GraphQLClient,
}

impl Data {
    /// Returns a new [`Data`] with an empty `reaction_roles` field and the passed-in `reload_handle`.
    fn new(reload_handle: ReloadHandle, root_url: String, api_key: String) -> Self {
        Data {
            reaction_roles: HashMap::new(),
            log_reload_handle: reload_handle,
            graphql_client: GraphQLClient::new(root_url, api_key),
        }
    }
}

/// Builds a [`poise::Framework`] with the given arguments and commands from [`commands::get_commands`].
#[instrument(level = "debug", skip(data))]
fn build_framework(
    owners: Option<UserId>,
    prefix_string: String,
    data: Data,
) -> Framework<Data, Error> {
    Framework::builder()
        .options(FrameworkOptions {
            commands: commands::get_commands(),
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            prefix_options: PrefixFrameworkOptions {
                prefix: Some(prefix_string),
                ..Default::default()
            },
            owners: owners.into_iter().collect(),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                scheduler::run_scheduler(ctx.clone(), data.graphql_client.clone()).await;
                Ok(data)
            })
        })
        .build()
}

fn prepare_data(config: &Config, reload_handle: ReloadHandle) -> Data {
    let mut data = Data::new(
        reload_handle,
        config.root_url.clone(),
        config.api_key.clone(),
    );
    data.populate_with_reaction_roles();
    data
}

async fn build_client(config: &Config, data: Data) -> Result<Client, anyhow::Error> {
    ClientBuilder::new(
        config.discord_token.clone(),
        GatewayIntents::non_privileged()
            | GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILD_MEMBERS,
    )
    .framework(build_framework(
        config.owner_id,
        config.prefix_string.clone(),
        data,
    ))
    .await
    .context("Failed to create the Serenity client")
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();
    let config = Config::default();

    let reload_handle = setup_tracing(config.debug, config.enable_debug_libraries)
        .context("Failed to setup tracing")?;

    info!(
        "Starting {} v{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    debug!(
        "Configuration loaded: debug={}, enable_debug_libraries={}, owner_id={:?}, prefix_string={}, root_url={}",
        config.debug, config.enable_debug_libraries, config.owner_id, config.prefix_string, config.root_url
    );

    let data = prepare_data(&config, reload_handle);
    let mut client = build_client(&config, data).await?;

    client
        .start()
        .await
        .context("Failed to start the Serenity client")?;

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
            handle_reaction(ctx, add_reaction, data, true).await?;
        }
        FullEvent::ReactionRemove { removed_reaction } => {
            handle_reaction(ctx, removed_reaction, data, false).await?;
        }

        FullEvent::GuildMemberRemoval {
            guild_id: _,
            user,
            member_data_if_available: Some(member),
        } => {
            let roles: Vec<String> = member
                .roles
                .iter()
                .filter(|r| r.get() != member.guild_id.get())
                .map(|r| r.get().to_string())
                .collect();

            if let Err(e) = data
                .graphql_client
                .save_member_roles(user.id.to_string(), roles)
                .await
            {
                println!("Failed to save roles: {:?}", e);
            }
        }
        FullEvent::GuildMemberAddition { new_member } => {
            let (exists, roles) = match data
                .graphql_client
                .get_member_roles(new_member.user.id.to_string())
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    println!("Failed to fetch roles: {:?}", e);
                    (false, vec![])
                }
            };

            if let Ok(member) = new_member.guild_id.member(ctx, new_member.user.id).await {
                // restore roles
                for role in roles {
                    if let Ok(role_id) = role.parse::<u64>() {
                        let _ = member.add_role(ctx, RoleId::new(role_id)).await;
                    }
                }
                // probated role
                if exists {
                    let _ = member.add_role(ctx, RoleId::new(1484798446228475905)).await;
                }
            }
        }
        _ => {}
    }

    Ok(())
}
