mod builder;
mod client;
mod command;
mod config;
mod handler;
mod parser;
mod status_monitor;
mod systemctl;
mod systemd_status;

use client::{ClientBuilder, ClientBuilderImpl};
use command::CommandRunnerImpl;
use config::ConfigImpl;
use handler::HandlerImpl;
use parser::ParserImpl;
use shaku::{module, HasComponent};
use status_monitor::StatusMonitorImpl;
use systemctl::SystemctlImpl;
use systemd_status::SystemdStatusManagerImpl;

module! {
    RootModule {
        components = [ClientBuilderImpl, CommandRunnerImpl, ConfigImpl, ParserImpl, StatusMonitorImpl, SystemctlImpl, SystemdStatusManagerImpl, HandlerImpl],
        providers = [],
    }
}

#[tokio::main]
async fn main() {
    let module = RootModule::builder()
        .with_component_parameters::<SystemdStatusManagerImpl>(
            systemd_status::make_params().await.unwrap(),
        )
        .build();

    let client_builder: &dyn ClientBuilder = module.resolve_ref();
    client_builder.build().await.unwrap().start().await.unwrap();
}
