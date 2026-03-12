use crate::{Data, Error};
use chrono::NaiveDate;
use poise::serenity_prelude as serenity;
use serenity::all::{CreateEmbed, CreateMessage};

#[poise::command(slash_command)]
pub async fn apply_leave(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "Start date (YYYY-MM-DD)"] start_date: NaiveDate,
    #[description = "Duration in days"] duration: Option<i32>,
    reason: String,
) -> Result<(), Error> {
    let discord_id = ctx.author().id.to_string();

    let duration = duration.unwrap_or(1);
    ctx.data()
        .graphql_client
        .apply_leave(&discord_id, start_date, duration, reason)
        .await?;

    let embed = if duration == 1 {
        CreateEmbed::new()
            .title("📝 Leave Request")
            .description(format!(
                "**User :** <@{}>\n\
                        **Start Date :** {}\n\
                        **Duration :** {} day\n\n\
                        ",
                discord_id, start_date, duration
            ))
    } else {
        CreateEmbed::new()
            .title("📝 Leave Request")
            .description(format!(
                "**User:** <@{}>\n\
                        **Start Date:** {}\n\
                        **Duration:** {} days\n\n\
                        React with ✅ to approve.",
                discord_id, start_date, duration
            ))
    };

    let message = ctx
        .channel_id()
        .send_message(ctx, CreateMessage::new().embed(embed))
        .await?;

    if duration != 1 {
        message
            .react(ctx, serenity::ReactionType::Unicode("✅".into()))
            .await?;

        message
            .react(ctx, serenity::ReactionType::Unicode("❌".into()))
            .await?;
    }
    ctx.say("Leave request submitted.").await?;

    Ok(())
}
