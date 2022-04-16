use crate::config::{Service, ServicePermission};
use crate::systemctl::{restart, start, status, stop, SystemctlError};
use futures::future::join_all;
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};

pub enum UserCommand<'a> {
    Start { service: &'a Service },
    Stop { service: &'a Service },
    Restart { service: &'a Service },
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
    pub async fn run(&self) -> Result<String, UserCommandError> {
        match self {
            UserCommand::Start { service } => {
                ensure_allowed(service, ServicePermission::Start)?;
                start(&service.unit).await?;
                Ok(format!("Started {}", service.name))
            }
            UserCommand::Stop { service } => {
                ensure_allowed(service, ServicePermission::Stop)?;
                stop(&service.unit).await?;
                Ok(format!("Stopped {}", service.name))
            }
            UserCommand::Restart { service } => {
                ensure_allowed(service, ServicePermission::Stop)?;
                ensure_allowed(service, ServicePermission::Start)?;
                restart(&service.unit).await?;
                Ok(format!("Restarted {}", service.name))
            }
            UserCommand::Status { services } => {
                for service in services {
                    ensure_allowed(service, ServicePermission::Status)?;
                }
                let statuses = join_all(services.iter().map(|service| status(&service.unit)))
                    .await
                    .iter()
                    .zip(services)
                    .map(|(status, service)| -> Result<String, SystemctlError> {
                        status.and_then(|status| format!("{} status: {}", &service.name, status))
                    })
                    .collect::<Result<Vec<String>, SystemctlError>>()?
                    .join("\n");
                Ok(statuses)
            }
        }
    }
}
