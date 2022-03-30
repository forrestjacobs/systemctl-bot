# <img src="./logo.png" height="24px" alt="systemctl-bot logo"> **systemctl-bot**: Control your Linux server from Discord

systemctl-bot lets you and your friends start and stop a subset of systemd services from Discord.

🚧 This is still very much a work in progress (both the documentation and the project itself.) As with most hobbyist projects, run this at your own risk.

## Why?

I wrote this bot because I wanted my friends to be able to log in to my small Minecraft server whenever they wanted, but I didn't want to run the Minecraft server jar 24x7. There are a bunch of control panels and the like for Minecraft, but all we really needed was an on/off switch for a few Minecraft worlds.

## Setup

 0. Make sure that the services you want to control are already system systemd units that are enabled on your Linux server. Right now, this bot only works with system units (as opposed to [user units](https://wiki.archlinux.org/title/Systemd/User).)

 1. [Create an application on the Discord Developer Portal](https://discord.com/developers/applications). (Feel free to use [the included logo](./logo.png).) After you've created it, jot down the _Application ID_.

 2. Go to _Bot_ > _Add Bot_. Fill it the page, and then jot down the bot's _token_.

 3. Go to _OAuth2_ > _URL Generator_. Check _bot_ and _applications.commands_. Navigate to the URL in the _Generated Link_ at the bottom of the page and follow the prompts in the Discord app.

 4. While you're in the Discord app, right-click on your server icon and select _Copy ID_. This is your _Guild ID_, which will be used in the next step.

 5. On the server you want to control, create `/etc/systemctl-bot/config.toml`:

    ```toml
    # Set these to the values you jotted down before
    application_id = 00000000
    guild_id = 00000000
    discord_token = "00000000.00000000.00000000"

    # Create a [[services]] section for each unit you want to control from Discord
    [[services]]
    name = "Minecraft" # How the service will appear in Discord
    unit = "minecraft-java-server.service"

    # You can list as many services as you want. They will appear in the same order in Discord's autocomplete list.
    [[services]]
    name = "Terraria"
    unit = "terraria.service"
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

 8. You can now start and stop services by typing `/systemctl <start|stop> [service name]` in your Discord server!
