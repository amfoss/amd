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
use crate::tasks::{get_tasks, Task};
use serenity::client::Context as SerenityContext;

use tokio::spawn;
use tokio::time::Duration;


/// Spawns a thread for each [`Task`].
///
/// [`SerenityContext`] is passed along with it so that they can
/// call any required Serenity functions without creating a new [`serenity::http`]
/// interface with a Discord token.
pub async fn run_scheduler(ctx: SerenityContext) {
    let tasks = get_tasks();

    for task in tasks {
        spawn(schedule_task(ctx.clone(), task));
    }
}

/// Runs the function [`Task::run`] and goes back to sleep until it's time to run again.
async fn schedule_task(ctx: SerenityContext, task: Box<dyn Task>) {
    loop {
        let next_run_in = task.run_in();
        tokio::time::sleep(next_run_in).await;
        task.run(ctx.clone()).await;
    }
}
