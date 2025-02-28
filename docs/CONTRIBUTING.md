# Contributing to amD

Thank you for considering contributing to our Discord bot written in Rust using the [`serenity`](https://github.com/serenity-rs/serenity) and [`Poise`](https://github.com/serenity-rs/poise) libraries. We welcome contributions of all kinds, including bug reports, feature requests, and code improvements. This document provides a guide for contributing effectively.

## Table of Contents

1. [Getting Started](#getting-started)
2. [How to Contribute](#how-to-contribute)
    - [Reporting Issues](#reporting-issues)
    - [Suggesting Features](#suggesting-features)
    - [Submitting Code Changes](#submitting-code-changes)
3. [Coding Standards](#coding-standards)
4. [Documentation](#documentation)

---

## Getting Started

### Prerequisites
Before proceeding, ensure you have the following:

1. Rust
```bash
# Download and install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh  

# Add Rust to the system PATH
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc  
source ~/.bashrc  
````
For more details, visit the [Official Rust installation page.](https://www.rust-lang.org/tools/install)

2. A [Discord Bot Token](https://discord.com/developers/).

### Setup

1. Clone the repository:
```bash
git clone https://github.com/amfoss/amd.git
cd amd
```

2. Create a `.env` file in the root directory and add the following:
```
DISCORD_TOKEN="your_discord_bot_token"

# (true for debugging, false for production)
ENABLE_DEBUG_LIBRARIES=true

# A numeric value
OWNER_ID="owners_id"

AMD_RUST_ENV="trace"  #trace for developers
```
3. Run the bot locally with `cargo run`. 

## How To Contribute

### Reporting Issues

If you encounter a bug, please check existing issues first to avoid duplicates. If none exist, create a new issue with the following details:

*  Title: Concise summary.
* Description: A detailed description of the issue.
*  Steps to Reproduce: If it's a bug, include steps to reproduce.
* Expected and Actual Behavior: Describe what you expected and what actually happened.

### Suggesting Features

We welcome ideas! Please open an issue titled "Feature Request: `<Feature Name>`" and provide:

* Problem: What problem does this feature solve?
* Solution: Describe how you envision it working.
* Alternatives Considered: Mention any alternatives you've considered.

### Submitting Code Changes

If you'd like to fix a bug, add a feature, or improve code quality:

* Check the open issues to avoid redundancy.
* Open a draft PR if you'd like feedback on an ongoing contribution.

## Coding Standards

* Follow Rust Conventions: Use idiomatic Rust patterns. Use `cargo fmt` and `cargo clippy` to format and lint your code.

* Modularity: Write modular, reusable functions. Avoid monolithic code.

* Descriptive Naming: Use descriptive names for variables, functions, and types.

## Documentation

### Command Handling

This bot uses `poise`, a command framework built on top of `serenity`. You can add commands in the `commands` module and get them registered using the `get_commands` function.

```rust
// Example command in src/commands.rs
#[poise::command(prefix_command)]
async fn amdctl(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("amD is up and running.").await?;
    Ok(())
}

pub fn get_commands() -> Vec<poise::Command<Data, Error>> {
    vec![amdctl()]
}
```

### Reaction Roles

amD supports automatic role assignment based on emoji reactions to specific messages. You can configure which messages and reactions trigger role assignemnt by modifying the `reaction_roles` Hashmap in the bot's `Data` struct in the `initialize_data()` function.

```rust
    reaction_roles: HashMap::new(),

    // Role IDs, use `\@<ROLE>` to get the ID on Discord
    let archive_role_id = RoleId::new(ARCHIVE_ROLE_ID);
    let mobile_role_id = RoleId::new(MOBILE_ROLE_ID);
    let systems_role_id = RoleId::new(SYSTEMS_ROLE_ID);
    ... /* excluded for brevity */

    let message_roles = [
        // Give y role if reacted with x emoji in a hashmap pair (x, y)
        (ReactionType::Unicode("📁".to_string()), archive_role_id),
        (ReactionType::Unicode("📱".to_string()), mobile_role_id),
        (ReactionType::Unicode("⚙️".to_string()), systems_role_id),
        ... /* excluded for brevity */

   ];

    data.reaction_roles.extend::<HashMap<ReactionType, RoleId>>(message_roles.into());

```

The event handler takes care of the rest:

```rust
        // On the event of a reaction being added
        FullEvent::ReactionAdd { add_reaction } => {
            let message_id = MessageId::new(ROLES_MESSAGE_ID);
            // Check if the reaction was added to the message we want and if it is reacted with the
            // emoji we want
            if add_reaction.message_id == message_id && data.reaction_roles.contains_key(&add_reaction.emoji) {
                    // Ensure it is in a server
                    if let Some(guild_id) = add_reaction.guild_id {
                        // Give the member the required role
                        if let Ok(member) =
                            guild_id.member(ctx, add_reaction.user_id.unwrap()).await
                        {
                            if let Err(e) = member.add_role(&ctx.http, data.reaction_roles.get(&add_reaction.emoji).expect("Hard coded value verified earlier.")).await {
                                eprintln!("Error: {:?}", e);
                            }
                        }
                    }
                }
        }
```

### Scheduler

The scheduler system allows you to easily define tasks that should be repeated periodically. Simply define a struct that implements the `task` trait and the `scheduler` module will automatically spawn a thread for your task on startup.

```rust
#[async_trait]
pub trait Task: Send + Sync {
    fn name(&self) -> &'static str;
    fn run_in(&self) -> Duration;
    async fn run(&self, ctx: Context);
}

```
Sample task that runs at 5 am every day:

```rust
pub struct StatusUpdateCheck;

#[async_trait]
impl Task for StatusUpdateCheck {
    fn name(&self) -> &'static str {
        "StatusUpdateCheck"
    }

    fn run_in(&self) -> Duration {
        time_until(5, 0)
    }

    async fn run(&self, ctx: Context) {
    ... /* Excluded for brevity */
    }
```
