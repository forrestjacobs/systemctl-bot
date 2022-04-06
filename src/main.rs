mod config;
mod handler;
mod register;

use crate::config::{get_config, Config};
use crate::handler::handle_global_command;
use crate::register::{are_commands_likely_registered, register_commands};

use rusty_interaction::handler::InteractionHandler;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config: Config = get_config();

    let mut handle = InteractionHandler::new(
        config.application_id,
        &config.public_key,
        Some(&config.discord_token),
    );
    handle.add_global_command("systemctl", handle_global_command);
    handle.add_data(config.services.clone());
    let r = handle.run(10443).await;

    if !are_commands_likely_registered(&config) {
        if let Err(e) = register_commands(&config).await {
            eprintln!("Error registering commands: {}", e);
        }
    }

    r
}
