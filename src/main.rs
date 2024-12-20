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
/// Stores all the commands for the bot.
mod commands;
/// Responsible for queries, models and mutation requests sent to and from
/// [root's](https://www.github.com/amfoss/root) graphql interace.
mod graphql;
/// Stores Discord IDs that are needed across the bot.
mod ids;
/// This module is a simple cron equivalent. It spawns threads for the regular [`Task`]s that need to be completed.
mod scheduler;
/// An interface to define a job that needs to be executed regularly, for example checking for status updates daily.
mod tasks;
/// Misc. helper functions that don't really have a place anywhere else.
mod utils;

use ids::{
    AI_ROLE_ID, ARCHIVE_ROLE_ID, DEVOPS_ROLE_ID, MOBILE_ROLE_ID, RESEARCH_ROLE_ID,
    ROLES_MESSAGE_ID, SYSTEMS_ROLE_ID, WEB_ROLE_ID,
};

use anyhow::Context as _;
use poise::{Context as PoiseContext, Framework, FrameworkOptions, PrefixFrameworkOptions};
use serenity::{
    all::{ReactionType, RoleId},
    client::{Context as SerenityContext, FullEvent},
    model::{gateway::GatewayIntents, id::MessageId},
};
use std::collections::HashMap;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = PoiseContext<'a, Data, Error>;

/// Runtime allocated storage for the bot.
pub struct Data {
    pub reaction_roles: HashMap<ReactionType, RoleId>,
}

/// This function is responsible for allocating the necessary fields
/// in [`Data`], before it is passed to the bot.
///
/// Currently, it only needs to store the (emoji, [`RoleId`]) pair used
/// for assigning roles to users who react to a particular message.
pub fn initialize_data() -> Data {
    let mut data = Data {
        reaction_roles: HashMap::new(),
    };

    // Define the emoji-role pairs
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

    // Populate reaction_roles map.
    data.reaction_roles
        .extend::<HashMap<ReactionType, RoleId>>(roles.into());

    data
}

/// Sets up the bot using a [`poise::Framework`], which handles most of the
/// configuration including the command prefix, the event handler, the available commands,
/// managing [`Data`] and running the [`scheduler`].
#[shuttle_runtime::main]
async fn main(
    #[shuttle_runtime::Secrets] secret_store: shuttle_runtime::SecretStore,
) -> shuttle_serenity::ShuttleSerenity {
    // Uses Shuttle's environment variable storage solution SecretStore
    // to access the token
    let discord_token = secret_store
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;

    let framework = Framework::builder()
        .options(FrameworkOptions {
            // Load bot commands
            commands: commands::get_commands(),
            // Pass the event handler function
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            // General bot settings, set to default except for prefix
            prefix_options: PrefixFrameworkOptions {
                prefix: Some(String::from("$")),
                ..Default::default()
            },
            ..Default::default()
        })
        // This function that's passed to setup() is called just as
        // the bot is ready to start.
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                let data = initialize_data();
                scheduler::run_scheduler(ctx.clone()).await;

                Ok(data)
            })
        })
        .build();

    let client = serenity::client::ClientBuilder::new(
        discord_token,
        GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT,
    )
    .framework(framework)
    .await
    .map_err(shuttle_runtime::CustomError::new)?;

    Ok(client.into())
}
/// Handles various events from Discord, such as reactions.
///
/// Current functionality includes:
/// - Adding roles to users based on reactions.
/// - Removing roles from users when their reactions are removed.
///
/// TODO: Refactor for better readability and modularity.
async fn event_handler(
    ctx: &SerenityContext,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        // Handle reactions being added.
        FullEvent::ReactionAdd { add_reaction } => {
            // Check if a role needs to be added i.e check if the reaction was added to [`ROLES_MESSAGE_ID`]
            if is_relevant_reaction(add_reaction.message_id, &add_reaction.emoji, data) {
                // This check for a guild_id isn't strictly necessary, since we're already checking
                // if the reaction was added to the [`ROLES_MESSAGE_ID`] which *should* point to a
                // message in the server.
                if let Some(guild_id) = add_reaction.guild_id {
                    if let Ok(member) = guild_id.member(ctx, add_reaction.user_id.unwrap()).await {
                        if let Err(e) = member
                            .add_role(
                                &ctx.http,
                                data.reaction_roles
                                    .get(&add_reaction.emoji)
                                    .expect("Hard coded value verified earlier."),
                            )
                            .await
                        {
                            // TODO: Replace with tracing
                            eprintln!("Error adding role: {:?}", e);
                        }
                    }
                }
            }
        }

        // Handle reactions being removed.
        FullEvent::ReactionRemove { removed_reaction } => {
            // Check if a role needs to be added i.e check if the reaction was added to [`ROLES_MESSAGE_ID`]
            if is_relevant_reaction(removed_reaction.message_id, &removed_reaction.emoji, data) {
                // This check for a guild_id isn't strictly necessary, since we're already checking
                // if the reaction was added to the [`ROLES_MESSAGE_ID`] which *should* point to a
                // message in the server.
                if let Some(guild_id) = removed_reaction.guild_id {
                    if let Ok(member) = guild_id
                        .member(ctx, removed_reaction.user_id.unwrap())
                        .await
                    {
                        if let Err(e) = member
                            .remove_role(
                                &ctx.http,
                                *data
                                    .reaction_roles
                                    .get(&removed_reaction.emoji)
                                    .expect("Hard coded value verified earlier."),
                            )
                            .await
                        {
                            eprintln!("Error removing role: {:?}", e);
                        }
                    }
                }
            }
        }

        // Ignore all other events for now.
        _ => {}
    }

    Ok(())
}

/// Helper function to check if a reaction was made to [`ROLES_MESSAGE_ID`] and if
/// [`Data::reaction_roles`] contains a relevant (emoji, role) pair.
fn is_relevant_reaction(message_id: MessageId, emoji: &ReactionType, data: &Data) -> bool {
    message_id == MessageId::new(ROLES_MESSAGE_ID) && data.reaction_roles.contains_key(emoji)
}
