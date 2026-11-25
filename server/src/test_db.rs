//! A migrated, throwaway Postgres schema for tests that need a real database.
//! Needs `DATABASE_URL` (CI provides one); never touches the real tables.

use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use uuid::Uuid;

pub struct TestDb {
    /// Connection whose search path is the test schema.
    pub db: DatabaseConnection,
    admin: DatabaseConnection,
    pub schema: String,
}

impl TestDb {
    pub async fn new() -> Self {
        let url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must point at a Postgres the tests may create schemas in");
        let schema = format!("t_{}", Uuid::new_v4().simple());

        let admin = Database::connect(&url).await.expect("connect to Postgres");
        admin
            .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
            .await
            .expect("create test schema");

        let mut opts = ConnectOptions::new(url);
        opts.set_schema_search_path(schema.clone())
            .max_connections(5);
        let db = Database::connect(opts)
            .await
            .expect("connect to test schema");
        migration::Migrator::up(&db, None)
            .await
            .expect("migrations apply to an empty schema");

        TestDb { db, admin, schema }
    }

    pub async fn cleanup(self) {
        let _ = self
            .admin
            .execute_unprepared(&format!("DROP SCHEMA {} CASCADE", self.schema))
            .await;
    }
}
