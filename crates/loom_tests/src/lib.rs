//! Shared test utilities for Loom integration tests.
//!
//! Each test creates an isolated PostgreSQL database, runs migrations,
//! and cleans up after itself. Set `TEST_DATABASE_URL` to a Postgres
//! connection string (e.g. `postgres://postgres:postgres@localhost/postgres`)
//! to enable integration tests.

use std::sync::Arc;

use sqlx::PgPool;

use loom_core::doctype::controller::NoopHookRunner;
use loom_core::doctype::meta::{DocFieldMeta, DocPermMeta, FieldType, Meta, NamingRule};
use loom_core::doctype::DocTypeRegistry;

/// Skip integration tests when TEST_DATABASE_URL is not set.
#[macro_export]
macro_rules! skip_without_db {
    () => {
        if std::env::var("TEST_DATABASE_URL").is_err() {
            eprintln!("Skipping: TEST_DATABASE_URL not set");
            return;
        }
    };
}

/// Create an isolated test database and return a pool connected to it.
/// The returned `TestDb` will drop the database when it goes out of scope.
pub struct TestDb {
    pub pool: PgPool,
    pub db_name: String,
    base_url: String,
}

impl TestDb {
    pub async fn new() -> Self {
        let base_url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set for integration tests");

        let db_name = format!(
            "loom_test_{}",
            uuid::Uuid::new_v4().simple().to_string().get(..12).unwrap()
        );

        // Connect to the admin database to create our test DB
        let admin_pool = PgPool::connect(&base_url).await.unwrap();
        sqlx::query(&format!("CREATE DATABASE \"{}\"", db_name))
            .execute(&admin_pool)
            .await
            .unwrap();
        admin_pool.close().await;

        // Build connection URL for the new test database
        let (base, _) = base_url.rsplit_once('/').expect("URL must contain /dbname");
        let test_url = format!("{}/{}", base, db_name);
        let pool = PgPool::connect(&test_url).await.unwrap();

        Self {
            pool,
            db_name,
            base_url,
        }
    }

    /// Run system table migrations and return a ready-to-use pool.
    pub async fn migrated() -> Self {
        let db = Self::new().await;
        loom_core::db::migrate::migrate_system_tables(&db.pool)
            .await
            .unwrap();
        db
    }

    /// Run full migrations including a DocType registry with the given metas.
    pub async fn with_doctypes(metas: Vec<Meta>) -> (Self, Arc<DocTypeRegistry>) {
        let db = Self::migrated().await;
        let registry = Arc::new(DocTypeRegistry::new());
        for meta in metas {
            registry.register(meta).await;
        }
        loom_core::db::migrate::migrate_all(&db.pool, &registry)
            .await
            .unwrap();
        (db, registry)
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        let base_url = self.base_url.clone();
        let db_name = self.db_name.clone();

        // We can't do async cleanup in Drop, so spawn a blocking task.
        // The database will be cleaned up eventually.
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                if let Ok(admin_pool) = PgPool::connect(&base_url).await {
                    // Terminate existing connections
                    let _ = sqlx::query(&format!(
                        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}'",
                        db_name
                    ))
                    .execute(&admin_pool)
                    .await;
                    let _ = sqlx::query(&format!("DROP DATABASE IF EXISTS \"{}\"", db_name))
                        .execute(&admin_pool)
                        .await;
                    admin_pool.close().await;
                }
            });
        });
    }
}

// ---------------------------------------------------------------------------
// Test fixture helpers
// ---------------------------------------------------------------------------

/// Build a minimal DocType meta for testing.
pub fn test_doctype(name: &str) -> Meta {
    Meta {
        name: name.to_string(),
        module: "Test".to_string(),
        naming_rule: NamingRule::Hash,
        fields: vec![
            DocFieldMeta {
                fieldname: "title".into(),
                label: Some("Title".into()),
                fieldtype: FieldType::Data,
                reqd: true,
                in_list_view: true,
                ..DocFieldMeta::default()
            },
            DocFieldMeta {
                fieldname: "description".into(),
                label: Some("Description".into()),
                fieldtype: FieldType::Text,
                ..DocFieldMeta::default()
            },
            DocFieldMeta {
                fieldname: "status".into(),
                label: Some("Status".into()),
                fieldtype: FieldType::Select,
                options: Some("Open\nClosed".into()),
                ..DocFieldMeta::default()
            },
            DocFieldMeta {
                fieldname: "priority".into(),
                label: Some("Priority".into()),
                fieldtype: FieldType::Int,
                ..DocFieldMeta::default()
            },
        ],
        permissions: vec![DocPermMeta {
            role: "Administrator".into(),
            read: true,
            write: true,
            create: true,
            delete: true,
            ..DocPermMeta::default()
        }],
        ..Meta::default()
    }
}

/// Build a submittable DocType meta for testing.
pub fn test_submittable_doctype(name: &str) -> Meta {
    let mut meta = test_doctype(name);
    meta.is_submittable = true;
    meta.permissions[0].submit = true;
    meta.permissions[0].cancel = true;
    meta
}

/// Build a DocType with restricted permissions for testing permlevel enforcement.
pub fn test_restricted_doctype(name: &str) -> Meta {
    Meta {
        name: name.to_string(),
        module: "Test".to_string(),
        naming_rule: NamingRule::Hash,
        fields: vec![
            DocFieldMeta {
                fieldname: "public_field".into(),
                label: Some("Public Field".into()),
                fieldtype: FieldType::Data,
                permlevel: 0,
                ..DocFieldMeta::default()
            },
            DocFieldMeta {
                fieldname: "secret_field".into(),
                label: Some("Secret Field".into()),
                fieldtype: FieldType::Data,
                permlevel: 1,
                ..DocFieldMeta::default()
            },
        ],
        permissions: vec![
            DocPermMeta {
                role: "All".into(),
                read: true,
                write: true,
                create: true,
                delete: true,
                ..DocPermMeta::default()
            },
            DocPermMeta {
                role: "System Manager".into(),
                read: true,
                write: true,
                permlevel: 1,
                ..DocPermMeta::default()
            },
        ],
        ..Meta::default()
    }
}

/// Build a DocType with Link field for testing link validation and fetch_from.
pub fn test_linked_doctype(name: &str, link_to: &str) -> Meta {
    Meta {
        name: name.to_string(),
        module: "Test".to_string(),
        naming_rule: NamingRule::Hash,
        fields: vec![
            DocFieldMeta {
                fieldname: "title".into(),
                label: Some("Title".into()),
                fieldtype: FieldType::Data,
                reqd: true,
                ..DocFieldMeta::default()
            },
            DocFieldMeta {
                fieldname: "linked_item".into(),
                label: Some("Linked Item".into()),
                fieldtype: FieldType::Link,
                options: Some(link_to.into()),
                ..DocFieldMeta::default()
            },
            DocFieldMeta {
                fieldname: "linked_title".into(),
                label: Some("Linked Title".into()),
                fieldtype: FieldType::Data,
                fetch_from: Some("linked_item.title".into()),
                ..DocFieldMeta::default()
            },
        ],
        permissions: vec![DocPermMeta {
            role: "Administrator".into(),
            read: true,
            write: true,
            create: true,
            delete: true,
            ..DocPermMeta::default()
        }],
        ..Meta::default()
    }
}

/// Create a system RequestContext for testing.
pub fn system_ctx(pool: &PgPool, registry: Arc<DocTypeRegistry>) -> loom_core::RequestContext {
    loom_core::RequestContext::system(pool.clone(), registry)
}

/// Create a non-admin RequestContext for testing.
pub fn user_ctx(
    pool: &PgPool,
    registry: Arc<DocTypeRegistry>,
    user: &str,
    roles: Vec<String>,
) -> loom_core::RequestContext {
    loom_core::RequestContext::new(
        user.to_string(),
        roles,
        "test".to_string(),
        pool.clone(),
        registry,
    )
}

/// Get the NoopHookRunner for tests that don't need hooks.
pub fn noop_hooks() -> NoopHookRunner {
    NoopHookRunner
}
