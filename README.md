<p align="center">
  <img src="logo.png" width="128" height="128" alt="Logo">
</p>

# systemctl-bot: Control your Linux server from Discord

systemctl-bot lets you and your friends start and stop a subset of systemd units from Discord.

🚧 This is still very much a work in progress (both the documentation and the project itself.) As with most hobbyist projects, run this at your own risk.

## Why?

I wrote this bot because I wanted my friends to be able to log in to my small Minecraft server whenever they wanted, but I didn't want to run the Minecraft server jar 24x7. There are a bunch of control panels and the like for Minecraft, but all we really needed was an on/off switch for a few Minecraft worlds.

## Setup

 0. Make sure that the services you want to control are already systemd units that are enabled on your Linux server. Right now, this bot only works with system units (as opposed to [user units](https://wiki.archlinux.org/title/Systemd/User).)

 1. [Create an application on the Discord Developer Portal](https://discord.com/developers/applications). (Feel free to use [the included logo](./logo.png).) After you've created it, jot down the _Application ID_.

 2. Go to _Bot_ > _Add Bot_. Fill it the page, and then jot down the bot's _token_.

 3. Go to _OAuth2_ > _URL Generator_. Check _bot_ and _applications.commands_. Navigate to the URL in the _Generated Link_ at the bottom of the page and follow the prompts in the Discord app.

 4. While you're in the Discord app, right-click on your server icon and select _Copy ID_. This is your _Guild ID_, which will be used in the next step.

 5. On the server you want to control, create `/etc/systemctl-bot.toml`:

    ```toml
    # Set these to the values you jotted down before
    application_id = 88888888
    guild_id = 88888888
    discord_token = "88888888.88888888.88888888"

    # Create a [[units]] section for each unit you want to control from Discord
    [[units]]
    name = "minecraft-java-server"
    permissions = ["start", "stop", "status"]

    # You can list as many units as you want. They will appear in the same order in Discord's autocomplete list.
    [[units]]
    name = "terraria"
    permissions = ["status"] # only allow status checking
    ```

 6. Check out this git repo and [build it using Cargo](https://doc.rust-lang.org/cargo/commands/cargo-build.html):

    ```sh
    # Assuming you already have git and cargo set up
    % cd /where/you/want/systemctl-bot/to/live
    % git clone https://github.com/forrestjacobs/systemctl-bot.git
    % cd systemctl-bot
    % cargo build --release
    ```

 7. Run the bot with enough priviledges for it to call systemctl. (Once you have this working, you'll probably want to set it up as a systemd service.)

    ```sh
    % sudo ./target/release/systemctl-bot
    ```

 8. You can now control units by typing `/systemctl <start|stop|restart|status> [unit name]` in your Discord server!

## Configuration

systemctl-bot reads its configuration from `/etc/systemctl-bot.toml`, unless you set a different path with the `--config` flag. This is a [TOML](https://toml.io/) file with the following keys:

| key              | type                                  | description                                                            |
| ---------------- | ------------------------------------- | ---------------------------------------------------------------------- |
| `application_id` | number                                | Application ID from the Discord Developer Portal. (See _Setup_ above.) |
| `guild_id`       | number                                | Your Discord server's guild ID. (See _Setup_ above.)                   |
| `discord_token`  | string                                | Bot token from the Discord Developer Portal. (See _Setup_ above.)      |
| `units`          | array of [units](#unit-configuration) | Units to control. See [unit configuration](#unit-configuration) below. |

### Unit configuration

| key           | type             | description                                                                   |
| ------------- | ---------------- | ----------------------------------------------------------------------------- |
| `name`        | string           | Name of the systemd unit. You can omit the `.service` for service units.      |
| `permissions` | array of strings | Array of allowed actions. Possible values are: "start", "stop", and "status". |
