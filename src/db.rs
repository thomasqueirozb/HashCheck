use crate::error::HashCheckError;
use rusqlite::{params, Connection};
use std::path::Path;

pub fn open<P: AsRef<Path>>(path: P) -> rusqlite::Result<(bool, Connection)> {
    let p = path.as_ref();

    let new = !p.exists();

    let conn = Connection::open(p)?;
    if new {
        conn.execute(
            "CREATE TABLE file (
                  path        TEXT PRIMARY KEY,
                  sha256sum   BLOB NOT NULL
             )",
            [],
        )?;
    }

    Ok((new, conn))
}

pub trait HashCheck {
    fn insert(&self, path: impl Into<String>, data: impl AsRef<[u8]>) -> rusqlite::Result<usize>;
    fn compare(&self, path: impl Into<String>, expected: impl AsRef<[u8]>) -> anyhow::Result<()>;
}

impl HashCheck for Connection {
    fn insert(&self, path: impl Into<String>, data: impl AsRef<[u8]>) -> rusqlite::Result<usize> {
        let r = self.execute(
            "INSERT INTO file (path, sha256sum) VALUES (?1, ?2)",
            params![path.into(), data.as_ref()],
        );
        r
    }

    fn compare(&self, path: impl Into<String>, found: impl AsRef<[u8]>) -> anyhow::Result<()> {
        let path = path.into();
        let found = found.as_ref();

        let mut stmt = self.prepare("SELECT sha256sum FROM file WHERE path = ?")?;
        let mut rows = stmt.query(params![path])?;
        let row = rows.next()?.ok_or(HashCheckError::Created(path.clone()))?;

        let expected: Vec<u8> = row.get(0)?;

        if expected != found {
            anyhow::bail!(HashCheckError::WrongHash {
                path,
                found: hex::encode(found),
                expected: hex::encode(expected),
            })
        }
        if rows.next()?.is_some() {
            anyhow::bail!(HashCheckError::Multiple(path))
        }

        Ok(())
    }
}
