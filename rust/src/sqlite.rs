use log::info;
use sqlx::{migrate::MigrateDatabase, Error, Sqlite, SqlitePool};
use std::fs;
use std::path::PathBuf;

pub async fn get_db_connection(db_url: &str) -> Result<SqlitePool, Error> {
    let db_pool = SqlitePool::connect(db_url).await?;
    Ok(db_pool)
}

pub async fn setup_db(db_url: &str) -> Result<(), Error> {
    if !Sqlite::database_exists(db_url).await.unwrap_or(false) {
        info!("Creating DB {}", db_url);
        match Sqlite::create_database(db_url).await {
            Ok(_) => info!("Created DB"),
            Err(error) => return Err(error),
        }
    } else {
        info!("DB exists")
    }

    let db = get_db_connection(db_url).await?;
    let result = sqlx::query(
        "
        CREATE TABLE IF NOT EXISTS file_uris (
            id INTEGER PRIMARY KEY,
            uri TEXT UNIQUE
        );
        CREATE TABLE IF NOT EXISTS vrs_locations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            vrs_id TEXT NOT NULL,
            chr TEXT NOT NULL,
            pos INTEGER NOT NULL,
            uri_id INTEGER NOT NULL,
            FOREIGN KEY (uri_id) REFERENCES file_uris(id),
            UNIQUE(vrs_id, chr, pos, uri_id)
        );
        ",
    )
    .execute(&db)
    .await?;
    info!("created table result: {:?}", result);
    Ok(())
}

pub fn cleanup_tempfiles(db_url: &str) -> Result<(), Error> {
    let owned_db_url = String::from(db_url);
    let db_path = owned_db_url.strip_prefix("sqlite://").unwrap();
    let mut db_pathbuf = PathBuf::from(db_path);
    db_pathbuf.set_extension("db-shm");
    let _ = fs::remove_file(db_pathbuf.clone());
    db_pathbuf.set_extension("db-wal");
    let _ = fs::remove_file(db_pathbuf);
    Ok(())
}

#[derive(Debug)]
pub struct DbRow {
    pub vrs_id: String,
    pub chr: String,
    pub pos: i64,
    pub uri_id: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, NamedTempFile};

    #[tokio::test]
    async fn test_setup_db() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_url = format!("sqlite://{}", temp_file.path().to_str().unwrap());
        setup_db(&db_url).await.expect("Setup DB failed");
    }

    #[test]
    fn test_cleanup_tempfiles() {
        let temp_dir = tempdir().expect("Failed to create temp file");
        let temp_path = temp_dir.path();
        let db_file_path = temp_path.join("test.sqlite");
        let db_url = format!("sqlite://{}", db_file_path.to_str().unwrap());
        let db_shm_path = temp_path.join("test.db-shm");
        let db_wal_path = temp_path.join("test.db-wal");
        fs::File::create(&db_shm_path).expect("Failed to create db-shm file");
        fs::File::create(&db_wal_path).expect("Failed to create db-wal file");
        assert!(db_shm_path.exists());
        assert!(db_wal_path.exists());
        cleanup_tempfiles(&db_url).expect("cleanup_tempfiles failed");
        assert!(!db_shm_path.exists());
        assert!(!db_wal_path.exists());
    }
}
