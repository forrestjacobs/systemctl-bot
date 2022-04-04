use crate::config::{Config, Service};

use rusty_interaction::handler::{HandlerResponse, InteractionHandler};
use rusty_interaction::slash_command;
use rusty_interaction::types::application::{
    ApplicationCommandInteractionData, ApplicationCommandInteractionDataOption,
};
use rusty_interaction::types::interaction::*;

use std::process::Command;

pub enum HandleError {
    MissingData,
    MissingSubcommand,
    UnexpectedSubcommand(String),
    MissingService,
    UnexpectedService(String),
    SystemctlError(Option<i32>, String),
    SystemctlIoError(std::io::Error),
}

impl HandleError {
    pub fn to_response_message(&self) -> String {
        match self {
            HandleError::SystemctlError(exit_code, stderr) => {
                format!(
                    "Error: {}{}",
                    stderr,
                    exit_code
                        .map(|c| format!(" (Exit code {})", c))
                        .unwrap_or_else(|| String::from(""))
                )
            }
            _ => String::from("Unexpected Error"),
        }
    }
}

fn map_systemctl_errors(
    output: Result<std::process::Output, std::io::Error>,
) -> Result<std::process::Output, HandleError> {
    let output = output.map_err(|e| HandleError::SystemctlIoError(e))?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(HandleError::SystemctlError(
            output.status.code(),
            String::from_utf8(output.stderr).unwrap(),
        ))
    }
}

fn get_service<'a, 'b>(
    command: &'a ApplicationCommandInteractionDataOption,
    option_index: usize,
    config: &'b Config,
) -> Result<(&'a String, &'b Service), HandleError> {
    let service_name = &command
        .options
        .as_ref()
        .ok_or_else(|| HandleError::MissingData)?
        .get(option_index)
        .ok_or_else(|| HandleError::MissingService)?
        .value;
    let service = config
        .services
        .get(service_name)
        .ok_or_else(|| HandleError::UnexpectedService(String::from(service_name)))?;
    Ok((service_name, service))
}

fn start(
    sub_command: &ApplicationCommandInteractionDataOption,
    config: &Config,
    ctx: &Context,
) -> Result<HandlerResponse, HandleError> {
    let (service_name, service) = get_service(sub_command, 0, config)?;
    map_systemctl_errors(
        Command::new("systemctl")
            .arg("start")
            .arg(&service.unit)
            .output(),
    )?;
    Ok(ctx
        .respond()
        .message(format!("Started {}", service_name))
        .finish())
}

fn stop(
    sub_command: &ApplicationCommandInteractionDataOption,
    config: &Config,
    ctx: &Context,
) -> Result<HandlerResponse, HandleError> {
    let (service_name, service) = get_service(sub_command, 0, config)?;
    map_systemctl_errors(
        Command::new("systemctl")
            .arg("stop")
            .arg(&service.unit)
            .output(),
    )?;
    Ok(ctx
        .respond()
        .message(format!("Stopped {}", service_name))
        .finish())
}

fn handle_interaction(
    data: Option<&ApplicationCommandInteractionData>,
    config: &Config,
    ctx: &Context,
) -> Result<HandlerResponse, HandleError> {
    let sub_command = data
        .ok_or_else(|| HandleError::MissingData)?
        .options
        .as_ref()
        .ok_or_else(|| HandleError::MissingData)?
        .get(0)
        .ok_or_else(|| HandleError::MissingSubcommand)?;
    match sub_command.name.as_str() {
        "start" => Ok(start(sub_command, config, ctx)?),
        "stop" => Ok(stop(sub_command, config, ctx)?),
        name => Err(HandleError::UnexpectedSubcommand(String::from(name))),
    }
}

#[slash_command]
#[defer]
pub async fn handle_global_command(
    handler: &mut InteractionHandler,
    ctx: Context,
) -> HandlerResponse {
    let data = ctx.interaction.data.as_ref();
    let config = handler.data.get::<Config>().unwrap();
    match handle_interaction(data, config, &ctx) {
        Ok(response) => response,
        // TODO: Log
        Err(e) => ctx.respond().message(e.to_response_message()).finish(),
    }
}
