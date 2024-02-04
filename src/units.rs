use bitflags::bitflags;
use indexmap::IndexMap;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct UnitPermissions: u32 {
        const Start  = 0b00000100;
        const Stop   = 0b00000010;
        const Status = 0b00000001;
    }
}

pub type Units = IndexMap<String, UnitPermissions>;

pub trait UnitsTrait {
    fn with_permissions<'a>(
        &'a self,
        permissions: UnitPermissions,
    ) -> impl Iterator<Item = &'a str>;
}

impl UnitsTrait for Units {
    fn with_permissions<'a>(
        &'a self,
        permissions: UnitPermissions,
    ) -> impl Iterator<Item = &'a str> {
        self.iter()
            .filter(move |(_, unit)| unit.contains(permissions))
            .map(|(name, _)| name.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;

    #[test]
    fn filter_units_by_permissions() {
        let units = Units::from([
            ("000".to_string(), UnitPermissions::empty()),
            ("001".to_string(), UnitPermissions::Status),
            ("010".to_string(), UnitPermissions::Stop),
            (
                "011".to_string(),
                UnitPermissions::Stop | UnitPermissions::Status,
            ),
            ("100".to_string(), UnitPermissions::Start),
            (
                "101".to_string(),
                UnitPermissions::Start | UnitPermissions::Status,
            ),
            (
                "110".to_string(),
                UnitPermissions::Start | UnitPermissions::Stop,
            ),
            ("111".to_string(), UnitPermissions::all()),
        ]);
        assert_eq!(
            units
                .with_permissions(UnitPermissions::Start | UnitPermissions::Stop)
                .collect_vec(),
            ["110", "111"]
        );
    }
}
