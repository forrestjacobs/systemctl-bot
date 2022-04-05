mod config;
mod handler;

use crate::config::get_config;
use crate::handler::handle_global_command;

use rusty_interaction::handler::InteractionHandler;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = get_config();
    let mut handle = InteractionHandler::new(
        config.application_id,
        &config.public_key,
        Some(&config.discord_token),
    );
    handle.add_global_command("systemctl", handle_global_command);
    handle.add_data(config);
    handle.run(10443).await
}
