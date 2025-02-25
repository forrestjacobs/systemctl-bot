mod client;
mod command;
mod config;
mod status_monitor;
mod systemctl;
mod systemd_status;

use client::{ClientBuilder, ClientBuilderImpl};
use command::DataImpl;
use config::ConfigImpl;
use shaku::{module, HasComponent};
use status_monitor::StatusMonitorImpl;
use systemctl::SystemctlImpl;
use systemd_status::SystemdStatusManagerImpl;

module! {
    RootModule {
        components = [ClientBuilderImpl, ConfigImpl, DataImpl, StatusMonitorImpl, SystemctlImpl, SystemdStatusManagerImpl],
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
