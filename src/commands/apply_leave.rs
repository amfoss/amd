use crate::{Data, Error};
use chrono::{Local, NaiveDate};
use poise::serenity_prelude as serenity;
use serenity::all::{CreateEmbed, CreateMessage};

#[poise::command(slash_command)]
pub async fn apply_leave(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "Start date (YYYY-MM-DD)"] start_date: NaiveDate,
    #[description = "Duration in days"] duration: Option<i32>,
    reason: String,
) -> Result<(), Error> {
    let discord_id = ctx.author().id.get().to_string();

    ctx.defer().await?;

    let duration = duration.unwrap_or(1);

    let today = Local::now().date_naive();

    if start_date < today {
        ctx.say("❌ Leave start date cannot be in the past.")
            .await?;
        return Ok(());
    }

    let embed = if duration == 1 {
        CreateEmbed::new()
            .title("📝 Leave Request")
            .description(format!(
                "**User:** <@{}>\n\
                        **Start Date:** {}\n\
                        **Duration:** {} day\n\
                        **Reason:** {}\n\n\
                        **The Leave Is Approved by Bot**
                        ",
                discord_id, start_date, duration, reason,
            ))
    } else {
        CreateEmbed::new()
            .title("📝 Leave Request")
            .description(format!(
                "**User:** <@{}>\n\
                        **Start Date:** {}\n\
                        **Duration:** {} days\n\
                        **Reason:** {}\n\n\
                        React with ✅ to approve.",
                discord_id, start_date, duration, reason,
            ))
    };

    let message = ctx
        .channel_id()
        .send_message(ctx, CreateMessage::new().embed(embed))
        .await?;

    let message_id = message.id.get();

    ctx.data()
        .graphql_client
        .apply_leave(&discord_id, &message_id, start_date, duration, &reason)
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
