use std::collections::{HashMap, HashSet};

use crate::{
    command::Command,
    config::{Unit, UnitPermission},
};

pub type CommandUnits<'unit> = HashMap<Command, Vec<&'unit str>>;

fn get_units_with_perms<const N: usize>(units: &[Unit], perms: [UnitPermission; N]) -> Vec<&str> {
    let perms = HashSet::from(perms);
    units
        .iter()
        .filter(|unit| perms.is_subset(&unit.permissions))
        .map(|unit| unit.name.as_str())
        .collect()
}

pub fn get_command_units(units: &[Unit]) -> CommandUnits<'_> {
    HashMap::from([
        (
            Command::Start,
            get_units_with_perms(units, [UnitPermission::Start]),
        ),
        (
            Command::Stop,
            get_units_with_perms(units, [UnitPermission::Stop]),
        ),
        (
            Command::Restart,
            get_units_with_perms(units, [UnitPermission::Start, UnitPermission::Stop]),
        ),
    ])
}

pub fn get_status_units(units: &[Unit]) -> Vec<&str> {
    get_units_with_perms(units, [UnitPermission::Status])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units_fixture() -> Vec<Unit> {
        vec![
            Unit {
                name: "restartable.service".to_owned(),
                permissions: HashSet::from([UnitPermission::Start, UnitPermission::Stop]),
            },
            Unit {
                name: "reportable.service".to_owned(),
                permissions: HashSet::from([UnitPermission::Status]),
            },
        ]
    }

    #[test]
    fn basic_command_units() {
        assert_eq!(
            get_command_units(&units_fixture()),
            HashMap::from([
                (Command::Start, vec!["restartable.service"]),
                (Command::Stop, vec!["restartable.service"]),
                (Command::Restart, vec!["restartable.service"]),
            ])
        );
    }

    #[test]
    fn basic_status_units() {
        assert_eq!(
            get_status_units(&units_fixture()),
            vec!["reportable.service"]
        );
    }
}
