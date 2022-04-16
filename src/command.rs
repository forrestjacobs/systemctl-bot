use crate::config::{Service, ServicePermission};
use crate::systemctl::{start, status, stop, SystemctlError};
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};

pub enum UserCommand<'a> {
    Start { service: &'a Service },
    Stop { service: &'a Service },
    Status { services: Vec<&'a Service> },
}

#[derive(Debug)]
pub enum UserCommandError {
    SystemctlError(SystemctlError),
    NotAllowed,
}

impl Display for UserCommandError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            UserCommandError::SystemctlError(e) => write!(f, "{}", e),
            UserCommandError::NotAllowed => {
                write!(f, "Command is not allowed")
            }
        }
    }
}

impl Error for UserCommandError {}

impl From<SystemctlError> for UserCommandError {
    fn from(error: SystemctlError) -> Self {
        UserCommandError::SystemctlError(error)
    }
}

fn ensure_allowed(
    service: &Service,
    permission: ServicePermission,
) -> Result<(), UserCommandError> {
    if service.permissions.contains(&permission) {
        Ok(())
    } else {
        Err(UserCommandError::NotAllowed)
    }
}

impl UserCommand<'_> {
    pub fn run(&self) -> Result<String, UserCommandError> {
        match self {
            UserCommand::Start { service } => {
                ensure_allowed(service, ServicePermission::Start)?;
                start(&service.unit)?;
                Ok(format!("Started {}", service.name))
            }
            UserCommand::Stop { service } => {
                ensure_allowed(service, ServicePermission::Stop)?;
                stop(&service.unit)?;
                Ok(format!("Stopped {}", service.name))
            }
            UserCommand::Status { services } => {
                let statuses = services
                    .iter()
                    .map(|service| -> Result<String, UserCommandError> {
                        ensure_allowed(service, ServicePermission::Status)?;
                        Ok(format!(
                            "{} status: {}",
                            service.name,
                            status(&service.unit)?
                        ))
                    })
                    .collect::<Result<Vec<String>, UserCommandError>>()?
                    .join("\n");
                Ok(statuses)
            }
        }
    }
}
