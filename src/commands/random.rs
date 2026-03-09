use crate::{
    ids::{FIRST_YEAR_ROLE_ID, SECOND_YEAR_ROLE_ID},
    Context, Error,
};
use rand::seq::IteratorRandom;
use serenity::all::{Mentionable as _, RoleId};

#[poise::command(prefix_command)]
pub async fn random(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx.guild_id().ok_or("No guild id")?;
    let members = guild.members(ctx.http(), None, None).await?;

    let selected: Vec<_> = members
        .into_iter()
        .filter(|m| {
            !m.user.bot
                && (m.roles.contains(&RoleId::new(FIRST_YEAR_ROLE_ID))
                    || m.roles.contains(&RoleId::new(SECOND_YEAR_ROLE_ID)))
        })
        .sample(&mut rand::rng(), 5);

    let ping_message = selected
        .iter()
        .map(|m| m.user.mention().to_string())
        .collect::<Vec<_>>()
        .join("\n");

    ctx.say(format!("Pinging 5 members: {}", ping_message))
        .await?;

    Ok(())
}
