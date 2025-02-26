use serenity::async_trait;
use serenity::model::prelude::*;
use serenity::prelude::*;
use std::collections::HashSet;
use std::fs::{OpenOptions};
use std::io::{self, Write};

#[async_trait]
impl Command for ToggleExclusionCommand {
    fn name(&self) -> &str {
        "toggle_exclusion"
    }

    fn description(&self) -> &str {
        "Toggles exclusion of a user from the status update report."
    }

    async fn execute(&self, ctx: &Context, msg: &Message) -> anyhow::Result<()> {
        let args: Vec<String> = msg.content.split_whitespace().map(|s| s.to_string()).collect();
        if args.len() < 2 {
            msg.reply(ctx, "Please specify a user to toggle exclusion").await?;
            return Ok(());
        }
        let user_id = args[1].parse::<u64>().unwrap_or(0);
        if user_id == 0 {
            msg.reply(ctx, "Invalid user ID").await?;
            return Ok(());
        }

        let mut excluded_members = get_excluded_members()?;
        if excluded_members.contains(&user_id) {
            excluded_members.remove(&user_id);
            msg.reply(ctx, "User has been removed from the exclusion list").await?;
        } else {
            excluded_members.insert(user_id);
            msg.reply(ctx, "User has been added to the exclusion list").await?;
        }

        save_excluded_members(excluded_members)?;
        Ok(())
    }
}
