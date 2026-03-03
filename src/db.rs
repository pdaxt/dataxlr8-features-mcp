use anyhow::Result;
use sqlx::PgPool;

/// Create the features schema in PostgreSQL if it doesn't exist.
pub async fn setup_schema(pool: &PgPool) -> Result<()> {
    sqlx::raw_sql(
        r#"
        CREATE SCHEMA IF NOT EXISTS features;

        CREATE TABLE IF NOT EXISTS features.flags (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL DEFAULT '',
            flag_type   TEXT NOT NULL DEFAULT 'global'
                        CHECK (flag_type IN ('global', 'page', 'feature')),
            enabled     BOOLEAN NOT NULL DEFAULT true,
            page_path   TEXT NOT NULL DEFAULT '',
            metadata    JSONB NOT NULL DEFAULT '{}',
            created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
            updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
        );

        CREATE TABLE IF NOT EXISTS features.flag_overrides (
            id             TEXT PRIMARY KEY,
            flag_id        TEXT NOT NULL REFERENCES features.flags(id) ON DELETE CASCADE,
            override_type  TEXT NOT NULL CHECK (override_type IN ('role', 'user')),
            target         TEXT NOT NULL,
            enabled        BOOLEAN NOT NULL DEFAULT true,
            created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
            updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
            UNIQUE (flag_id, override_type, target)
        );

        CREATE INDEX IF NOT EXISTS idx_flags_name ON features.flags(name);
        CREATE INDEX IF NOT EXISTS idx_overrides_flag_id ON features.flag_overrides(flag_id);
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}
