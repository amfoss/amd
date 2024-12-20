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
mod status_update;

use crate::{tasks::status_update::check_status_updates, utils::time::time_until};

use async_trait::async_trait;
use serenity::client::Context;
use tokio::time::Duration;

/// A [`Task`] is any job that needs to be executed on a regular basis.
/// A task has a function [`Task::run_in`] that returns the time till the
/// next ['Task::run`] is run. It also has a [`Task::name`] that can be used
/// in the future to display to the end user.
#[async_trait]
pub trait Task: Send + Sync {
    fn name(&self) -> &'static str;
    fn run_in(&self) -> Duration;
    async fn run(&self, ctx: Context);
}

/// Analogous to [`crate::commands::get_commands`], every task that is defined
/// must be included in the returned vector in order for it to be scheduled.
pub fn get_tasks() -> Vec<Box<dyn Task>> {
    vec![Box::new(StatusUpdateCheck)]
}

/// Checks for status updates daily at 9 AM.
pub struct StatusUpdateCheck;

#[async_trait]
impl Task for StatusUpdateCheck {
    fn name(&self) -> &'static str {
        "StatusUpdateCheck"
    }

    fn run_in(&self) -> Duration {
        time_until(9, 0)
    }

    async fn run(&self, ctx: Context) {
        check_status_updates(ctx).await;
    }
}
