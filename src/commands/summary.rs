use crate::graphql::models::{LeaveCountRecord, MemberSummary};
use crate::ids::THE_LAB_CHANNEL_ID;
use crate::{Data, Error};
use chrono::{Datelike, Local, NaiveDate};
use poise::serenity_prelude::User;
use serenity::all::{ChannelId, CreateEmbed, CreateMessage};

#[poise::command(slash_command)]
pub async fn member_summary(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "Mention the member"] member: User,
    #[description = "Start Date (YYYY-MM-DD)"] start_date: Option<NaiveDate>,
    #[description = "End Date (YYYY-MM-DD)"] end_date: Option<NaiveDate>,
) -> Result<(), Error> {
    let discord_id = member.id.get().to_string();

    // take present date automatically and give this month's date and summary if both
    let (start_date, end_date) = match (start_date, end_date) {
        (Some(s), Some(e)) => (s, e),

        (None, None) => {
            let time = Local::now();
            let year = time.year();
            let month = time.month();
            let end = time.date_naive();

            let (target_year, target_month) = if end.day() == 1 {
                if month == 1 {
                    (year - 1, 12)
                } else {
                    (year, month - 1)
                }
            } else {
                (year, month)
            };

            let start = NaiveDate::from_ymd_opt(target_year, target_month, 1)
                .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;

            (start, end)
        }
        _ => {
            return Err(
                anyhow::anyhow!("Either provide both start and end dates, or none.").into(),
            );
        }
    };

    let leaves: LeaveCountRecord = ctx
        .data()
        .graphql_client
        .fetch_leaves(&discord_id, start_date, end_date)
        .await?;

    let summary: MemberSummary = ctx
        .data()
        .graphql_client
        .fetch_member_summary(&discord_id, start_date, end_date)
        .await?;

    let embed = CreateEmbed::new()
        .title("Member Summary📋")
        .description(format!(
            "**Report of** <@{}>
         • Period: **{} → {}**\n\n\
        **Attendance 📊**\n\
         • Presence: **{:.1}%**\n\n\
        **Updates 📝**\n\
         • Consistency: **{:.1}%** \n\n\
         **Leave Summary 📄**\n\
         • Total Leaves: **{}**",
            discord_id,
            start_date,
            end_date,
            summary.present_percent,
            summary.updates_percent,
            leaves.leave_count
        ));

    let lab_channel_id = ChannelId::new(THE_LAB_CHANNEL_ID);
    lab_channel_id
        .send_message(ctx.http(), CreateMessage::new().add_embed(embed))
        .await?;

    Ok(())
}
