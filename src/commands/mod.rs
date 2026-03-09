mod random;
mod set_log_level;

use crate::commands::random::random;
use crate::commands::set_log_level::set_log_level;
use serenity::all::RoleId;
use tracing::{debug, instrument};

use crate::{
    ids::{FOURTH_YEAR_ROLE_ID, THIRD_YEAR_ROLE_ID},
    Context, Data, Error,
};

/// Checks if the author has the Fourth Year or Third Year role. Can be used as an authorization procedure for other commands.
#[allow(dead_code)]
async fn is_privileged(ctx: &Context<'_>) -> bool {
    if let Some(guild_id) = ctx.guild_id() {
        if let Ok(member) = guild_id.member(ctx, ctx.author().id).await {
            return member.roles.contains(&RoleId::new(FOURTH_YEAR_ROLE_ID))
                || member.roles.contains(&RoleId::new(THIRD_YEAR_ROLE_ID));
        }
    }

    false
}

#[poise::command(prefix_command)]
#[instrument(level = "debug", skip(ctx))]
async fn amdctl(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("amD is up and running.").await?;
    Ok(())
}

/// Returns a vector containg [Poise Commands][`poise::Command`]
pub fn get_commands() -> Vec<poise::Command<Data, Error>> {
    let commands = vec![amdctl(), set_log_level(), random()];
    debug!(commands = ?commands.iter().map(|c| &c.name).collect::<Vec<_>>());
    commands
}
