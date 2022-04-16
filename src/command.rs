use crate::systemctl::{SystemctlError, stop, start};
use crate::config::Service;

pub enum UserCommand<'a> {
    Start { service: &'a Service },
    Stop { service: &'a Service },
}

impl UserCommand<'_> {
    pub fn run(&self) -> Result<String, SystemctlError> {
        match self {
            UserCommand::Start { service } => {
                start(&service.unit)?;
                Ok(format!("Started {}", service.name))
            }
            UserCommand::Stop { service } => {
                stop(&service.unit)?;
                Ok(format!("Stopped {}", service.name))
            }
        }
    }
}
