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
mod lab_attendance;
mod status_update;

use anyhow::Result;
use async_trait::async_trait;
use lab_attendance::PresenseReport;
use serenity::client::Context;
use status_update::StatusUpdateCheck;
use tokio::time::Duration;

/// A [`Task`] is any job that needs to be executed on a regular basis.
/// A task has a function [`Task::run_in`] that returns the time till the
/// next ['Task::run`] is run.
#[async_trait]
pub trait Task: Send + Sync {
    fn name(&self) -> &str;
    fn run_in(&self) -> Duration;
    async fn run(&self, ctx: Context) -> Result<()>;
}

/// Analogous to [`crate::commands::get_commands`], every task that is defined
/// must be included in the returned vector in order for it to be scheduled.
pub fn get_tasks() -> Vec<Box<dyn Task>> {
    vec![Box::new(StatusUpdateCheck), Box::new(PresenseReport)]
}
