use crate::config::Service;
use crate::systemctl::{start, status, stop, SystemctlError};

pub enum UserCommand<'a> {
    Start { service: &'a Service },
    Stop { service: &'a Service },
    Status { services: Vec<&'a Service> },
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
            UserCommand::Status { services } => {
                let statuses = services
                    .iter()
                    .map(|service| -> Result<String, SystemctlError> {
                        Ok(format!("{} status: {}", service.name, status(&service.unit)?))
                    })
                    .collect::<Result<Vec<String>, SystemctlError>>()?
                    .join("\n");
                Ok(statuses)
            }
        }
    }
}
