use std::fs::DirEntry;

use migrate::configuration::{parse_arguments, Direction};
use rusqlite::Connection;

#[derive(Debug)]
struct Migration {
    id: usize,
    entry: DirEntry,
}

impl Migration {
    fn new(id: usize, entry: DirEntry) -> Self {
        Migration { id, entry }
    }
}

fn main() {
    println!("Migrating db...");
    let arguments = parse_arguments();
    let database_path = arguments.database_connection;
    let migrations_directory = arguments.migration_path;
    let mut migrations: Vec<Migration> = vec![];
    let mut connection = Connection::open(database_path).unwrap();
    let directory_info = std::fs::read_dir(&migrations_directory).unwrap();
    connection
        .prepare("CREATE TABLE IF NOT EXISTS Migration (Version INT)")
        .unwrap()
        .execute([])
        .unwrap();
    let current_version: usize = connection
        .query_row("SELECT Version FROM Migration", [], |row| row.get(0))
        .unwrap_or_else(|_| {
            connection
                .prepare("INSERT INTO Migration VALUES(0)")
                .unwrap()
                .execute([])
                .unwrap();
            0
        });

    for entry in directory_info {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let mut name_parts = name.to_str().unwrap().split("_");
        let id: usize = name_parts.next().unwrap().parse().unwrap();
        let entry_direction = match name_parts
            .next()
            .unwrap()
            .split(".")
            .next()
            .unwrap()
            .to_lowercase()
            .as_str()
        {
            "up" => Direction::Up,
            "down" => Direction::Down,
            _ => panic!("Invalid direction"),
        };

        if entry_direction != arguments.migration_direction {
            continue;
        }

        migrations.push(Migration::new(id, entry));
    }

    match arguments.migration_direction {
        Direction::Up => migrations.sort_by(|a, b| a.id.cmp(&b.id)),
        Direction::Down => migrations.sort_by(|a, b| b.id.cmp(&a.id)),
    }

    let target_version = if let Some(version) = arguments.target_version {
        version
    } else {
        match arguments.migration_direction {
            Direction::Up => migrations.last().unwrap().id,
            Direction::Down => migrations.last().unwrap().id - 1,
        }
    };

    let transaction = connection.transaction().unwrap();

    for migration in migrations {
        match arguments.migration_direction {
            Direction::Up if migration.id <= current_version => continue,
            Direction::Up if migration.id > target_version => continue,
            Direction::Down if migration.id > current_version => continue,
            Direction::Down if migration.id <= target_version => continue,
            _ => (),
        }

        println!(
            "Applying {}",
            migration
                .entry
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
        );
        let sql = std::fs::read_to_string(migration.entry.path()).unwrap();
        transaction.execute_batch(&sql).unwrap();
    }

    transaction
        .execute("UPDATE Migration SET Version = ?1", [target_version])
        .unwrap();
    transaction.commit().unwrap();
    println!("Migration completed successfully");
}
