use crate::{
    ids::{FIRST_YEAR_ROLE_ID, SECOND_YEAR_ROLE_ID, THIRD_YEAR_ROLE_ID},
    Context, Error,
};
use rand::seq::IteratorRandom;
use serenity::all::{Mentionable as _, Role, RoleId, UserId};
use std::collections::HashSet;

#[poise::command(slash_command)]
pub async fn random(
    ctx: Context<'_>,
    count: Option<u32>,
    role1: Option<Role>,
    role2: Option<Role>,
    role3: Option<Role>,
) -> Result<(), Error> {
    let guild = ctx.guild_id().ok_or("No guild id")?;
    let members = guild.members(ctx.http(), None, None).await?;

    let count = count.unwrap_or(3) as usize;
    let mut selected_roles: HashSet<RoleId> = [role1, role2, role3]
        .into_iter()
        .flatten()
        .map(|role| role.id)
        .collect();

    if selected_roles.is_empty() {
        selected_roles.extend([
            RoleId::new(FIRST_YEAR_ROLE_ID),
            RoleId::new(SECOND_YEAR_ROLE_ID),
            RoleId::new(THIRD_YEAR_ROLE_ID),
        ]);
    }

    let eligible_members: Vec<_> = members // Filtering out bots and other ineligible members
        .into_iter()
        .filter(|m| !m.user.bot && (m.roles.iter().any(|role| selected_roles.contains(role))))
        .collect();

    if eligible_members.is_empty() {
        ctx.say("No eligible members found.").await?;
        return Ok(());
    }

    let recent_picks = {
        // Accessing recently picked members to avoid repetition
        let data = ctx.data();
        let recent_random_picks = data.recent_random_picks.lock().unwrap();
        recent_random_picks.clone()
    };

    let available_members: Vec<_> = eligible_members // Members who haven't been picked recently
        .iter()
        .filter(|member| !recent_picks.contains(&member.user.id))
        .collect();

    /* Since ThreadRng is not Send, keeping it alive across an .await causes command future to become non-Send.
    So, we are enclosing the selection in it's own scope so the ThreadRng is dropped before we hit any .await,
    allowing the command future to remain Send
     */

    let selected = {
        let mut rng = rand::thread_rng();

        let mut selected: Vec<_> = available_members
            .into_iter()
            .choose_multiple(&mut rng, count);

        if selected.len() < count {
            // If not enough members are available, fetch recently picked members
            let remaining_needed = count - selected.len();

            let selected_ids: HashSet<UserId> = selected.iter().map(|m| m.user.id).collect();

            let additional: Vec<_> = eligible_members
                .iter()
                .filter(|m| !selected_ids.contains(&m.user.id))
                .choose_multiple(&mut rng, remaining_needed);

            selected.extend(additional);
        }
        selected
    };

    let ping_message = selected
        .iter()
        .map(|m| m.user.mention().to_string())
        .collect::<Vec<_>>()
        .join("\n");

    {
        let data = ctx.data();
        let mut recent_random_picks = data.recent_random_picks.lock().unwrap();
        recent_random_picks.extend(selected.iter().map(|m| m.user.id)); // Adding selected members to recently picked set

        let eligible_ids: HashSet<UserId> = eligible_members.iter().map(|m| m.user.id).collect();
        recent_random_picks.retain(|user_id| eligible_ids.contains(user_id));
        if recent_random_picks.len() >= eligible_ids.len() {
            // If all eligible members have been picked atleast once, clear the recently picked set
            recent_random_picks.clear();
        }
    } // guard dropped here, mutex unlocked
    ctx.data().save_recent_picks();
    ctx.say(format!(
        "Pinging {} members: {}",
        selected.len(),
        ping_message
    ))
    .await?;

    Ok(())
}
