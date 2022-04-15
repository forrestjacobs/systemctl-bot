use crate::systemctl::{SystemctlError, stop, start};
use crate::config::Service;

pub struct ServiceEntry<'a> {
    pub name: String,
    pub value: &'a Service,
}

pub enum UserCommand<'a> {
    Start { service: ServiceEntry<'a> },
    Stop { service: ServiceEntry<'a> },
}

impl UserCommand<'_> {
    pub fn run(&self) -> Result<String, SystemctlError> {
        match self {
            UserCommand::Start { service } => {
                start(&service.value.unit)?;
                Ok(format!("Started {}", service.name))
            }
            UserCommand::Stop { service } => {
                stop(&service.value.unit)?;
                Ok(format!("Stopped {}", service.name))
            }
        }
    }
}
