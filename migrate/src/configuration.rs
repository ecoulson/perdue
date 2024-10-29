use std::str::FromStr;

use crate::cli::{
    read_command, read_flags, Command, FlagConfiguration, FlagMap, ToFlagConfiguration,
};

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
enum MigrationFlag {
    MigrationPath,
    DatabasePath,
    TargetVersion,
}

impl ToFlagConfiguration for MigrationFlag {
    fn to_config(&self) -> FlagConfiguration {
        match self {
            MigrationFlag::DatabasePath => FlagConfiguration::required("database_path", None, ""),
            MigrationFlag::MigrationPath => FlagConfiguration::required("migration_path", None, ""),
            MigrationFlag::TargetVersion => FlagConfiguration::optional("target_version", None, ""),
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum Direction {
    Up,
    Down,
}

impl FromStr for Direction {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string.to_lowercase().as_str() {
            "up" => Ok(Direction::Up),
            "down" => Ok(Direction::Down),
            _ => Err(String::new()),
        }
    }
}

#[derive(Debug)]
pub struct CliArguments {
    pub migration_direction: Direction,
    pub database_connection: String,
    pub migration_path: String,
    pub target_version: Option<usize>,
}

pub fn parse_arguments() -> CliArguments {
    let mut args = std::env::args().skip(1).peekable();
    let direction: Command<Direction> = read_command(&mut args).unwrap();
    let flags: FlagMap<MigrationFlag> = read_flags(
        &mut args,
        &vec![
            MigrationFlag::MigrationPath,
            MigrationFlag::DatabasePath,
            MigrationFlag::TargetVersion,
        ],
    );

    CliArguments {
        migration_direction: direction.value,
        migration_path: flags.get(MigrationFlag::MigrationPath),
        database_connection: flags.get(MigrationFlag::DatabasePath),
        target_version: flags.get_optional(MigrationFlag::TargetVersion),
    }
}
