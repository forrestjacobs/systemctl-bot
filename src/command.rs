use core::{
    error::Error,
    fmt::{self, Display, Formatter},
    str::FromStr,
};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
#[expect(
    clippy::arbitrary_source_item_ordering,
    reason = "Commands are intentionally ordered"
)]
pub enum Command {
    Start,
    Stop,
    Restart,
}

impl Command {
    pub const VALUES: [Self; 3] = [Self::Start, Self::Stop, Self::Restart];
}

#[derive(Debug, PartialEq, Eq)]
pub struct UnexpectedCommandError(pub String);

impl Display for UnexpectedCommandError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Unexpected command {}", self.0)
    }
}

impl Error for UnexpectedCommandError {}

impl FromStr for Command {
    type Err = UnexpectedCommandError;
    fn from_str(str: &str) -> Result<Self, Self::Err> {
        match str {
            "start" => Ok(Self::Start),
            "stop" => Ok(Self::Stop),
            "restart" => Ok(Self::Restart),
            str => Err(UnexpectedCommandError(str.to_owned())),
        }
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Start => write!(f, "start"),
            Self::Stop => write!(f, "stop"),
            Self::Restart => write!(f, "restart"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_unexpected_command_error() {
        assert_eq!(
            UnexpectedCommandError("recombobulate".to_owned()).to_string(),
            "Unexpected command recombobulate"
        );
    }

    #[test]
    fn command_from_str() {
        assert_eq!(Command::from_str("start"), Ok(Command::Start));
        assert_eq!(Command::from_str("stop"), Ok(Command::Stop));
        assert_eq!(Command::from_str("restart"), Ok(Command::Restart));
        assert_eq!(
            Command::from_str("recombobulate"),
            Err(UnexpectedCommandError("recombobulate".to_owned()))
        );
    }

    #[test]
    fn display_command() {
        assert_eq!(Command::Start.to_string(), "start");
        assert_eq!(Command::Stop.to_string(), "stop");
        assert_eq!(Command::Restart.to_string(), "restart");
    }
}
