# amFOSS Daemon

Discord bot used for the official amFOSS server for members. Built with [Serenity](https://www.github.com/serenity-rs/serenity) and [Poise](ttps://www.github.com/serenity-rs/poise).

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

2. Create a `.env` file in the root directory. You can refer `.env.sample` for the required variables.

3. Run the bot locally with `cargo run`.

## Contributing

Refer [CONTRIBUTING.md](/docs/CONTRIBUTING.md).

## License
This project is licensed under the GNU General Public License v3.0. See the LICENSE file for details.
