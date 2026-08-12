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

use serenity::all::UserId;

/// Environment variables for amD
///
/// # Fields
///
/// * debug: a boolean flag that decides in what context the application will be running on. When true, it is assumed to be in development. This allows us to filter out logs from `stdout` when in production. Defaults to false if not set.
/// * enable_debug_libraries: Boolean flag that controls whether tracing will output logs from other crates used in the project. This is only needed for really serious bugs. Defaults to false if not set.
/// * discord_token: The bot's discord token obtained from the Discord Developer Portal. The only mandatory variable required.
/// * owner_id: Used to allow access to privileged commands to specific users. If not passed, will set the bot to have no owners.
/// * prefix_string: The prefix used to issue commands to the bot on Discord. Always set to "$".
pub struct Config {
    pub debug: bool,
    pub enable_debug_libraries: bool,
    pub discord_token: String,
    pub owner_id: Option<UserId>,
    pub prefix_string: String,
    pub root_url: String,
    pub api_key: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            debug: parse_bool_env("DEBUG"),
            enable_debug_libraries: parse_bool_env("ENABLE_DEBUG_LIBRARIES"),
            discord_token: std::env::var("DISCORD_TOKEN")
                .expect("DISCORD_TOKEN was not found in env"),
            owner_id: parse_owner_id_env("OWNER_ID"),
            prefix_string: String::from("$"),
            root_url: std::env::var("ROOT_URL").expect("ROOT_URL was not found in env"),
            api_key: std::env::var("AMD_API_KEY").expect("AMD_API_KEY was not found in env"),
        }
    }
}

/// Tries to access the environment variable through the key passed in. If it is set, it will try to parse it as u64 and if that fails, it will log the error and return the default value None. If it suceeds the u64 parsing, it will convert it to a UserId and return Some(UserId). If the env. var. is not set, it will return None.
fn parse_owner_id_env(key: &str) -> Option<UserId> {
    std::env::var(key)
        .ok()
        .and_then(|s| {
            s.parse::<u64>()
                .map_err(|_| eprintln!("WARNING: Invalid OWNER_ID value '{s}', ignoring."))
                .ok()
        })
        .map(UserId::new)
}

/// Tries to access the environment variable through the key passed in. If it is set but an invalid boolean, it will log an error through tracing and default to false. If it is not set, it will default to false.
fn parse_bool_env(key: &str) -> bool {
    std::env::var(key)
        .map(|val| {
            val.parse().unwrap_or_else(|_| {
                eprintln!("Warning: Invalid DEBUG value '{val}', defaulting to false");
                false
            })
        })
        .unwrap_or(false)
}
