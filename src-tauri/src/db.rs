use std::time::SystemTime;

use sqlx::{sqlite::SqliteRow, Executor, Pool, Row, Sqlite};

use crate::key::Key;

impl Key {
    pub(crate) async fn db_select_all(db: &Pool<Sqlite>) -> Result<Vec<Self>, sqlx::Error> {
        let query = sqlx::query("SELECT * FROM keys");
        let rows = db.fetch_all(query).await?;
        rows.iter().map(Self::from_row).collect()
    }

    pub(crate) async fn db_select(
        db: &Pool<Sqlite>,
        item: &str,
        username: &str,
    ) -> Result<Self, sqlx::Error> {
        let query = sqlx::query("SELECT * FROM keys WHERE item = $1 AND username = $2")
            .bind(item)
            .bind(username);
        let row = db.fetch_one(query).await?;
        Self::from_row(&row)
    }

    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        let item: &str = row.try_get("item")?;
        let username: &str = row.try_get("username")?;
        let key: &str = row.try_get("key")?;
        Ok(Self {
            item: item.to_string(),
            username: username.to_string(),
            key: key.to_string(),
        })
    }

    pub(crate) async fn db_insert(
        &self,
        db: &Pool<Sqlite>,
    ) -> Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error> {
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let query = sqlx::query(
            r#"
INSERT INTO keys ( item, username, key, created_at, updated_at )
VALUES ( $1, $2, $3, $4, $5 )
        "#,
        )
        .bind(&self.item)
        .bind(&self.username)
        .bind(&self.key)
        .bind(ts as i64)
        .bind(ts as i64);
        db.execute(query).await
    }
}
