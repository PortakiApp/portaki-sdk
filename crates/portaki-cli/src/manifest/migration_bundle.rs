//! Builds `migrations.bundle.json` from `db/migrations/*.sql` (modules DB, applied at install).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

/// OCI / install artifact consumed by `ModuleInstallMigrationApplier`.
#[derive(Debug, Serialize)]
pub struct ModuleMigrationBundle {
    pub module_id: String,
    pub schema_version: String,
    pub revisions: Vec<MigrationRevision>,
}

#[derive(Debug, Serialize)]
pub struct MigrationRevision {
    pub revision: String,
    pub sql: String,
}

/// Writes `migrations.bundle.json` when `db/migrations/` exists.
pub fn write_migration_bundle(
    module_root: &Path,
    out_dir: &Path,
    module_id: &str,
    schema_version: u32,
) -> Result<Option<PathBuf>> {
    let migrations_dir = module_root.join("db/migrations");
    if !migrations_dir.is_dir() {
        return Ok(None);
    }

    let mut sql_files: Vec<PathBuf> = fs::read_dir(&migrations_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("sql"))
        .collect();
    sql_files.sort();

    if sql_files.is_empty() {
        return Ok(None);
    }

    let mut revisions = Vec::with_capacity(sql_files.len());
    for path in sql_files {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .context("migration file name utf-8")?;
        let revision = file_name.trim_end_matches(".sql").to_string();
        let sql = fs::read_to_string(&path)
            .with_context(|| format!("read migration {}", path.display()))?;
        revisions.push(MigrationRevision { revision, sql });
    }

    let bundle = ModuleMigrationBundle {
        module_id: module_id.to_string(),
        schema_version: schema_version.to_string(),
        revisions,
    };

    let dest = out_dir.join("migrations.bundle.json");
    let json = serde_json::to_string_pretty(&bundle).context("serialize migrations.bundle.json")?;
    fs::write(&dest, format!("{json}\n")).context("write migrations.bundle.json")?;
    Ok(Some(dest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    use tempfile::tempdir;

    #[test]
    fn when_db_migrations_exist_then_writes_bundle() {
        let root = tempdir().unwrap();
        let migrations = root.path().join("db/migrations");
        fs::create_dir_all(&migrations).unwrap();
        let mut first =
            fs::File::create(migrations.join("20260526100000_v1_baseline.sql")).unwrap();
        writeln!(first, "CREATE TABLE foo (id INT);").unwrap();
        let out = root.path().join("out");
        fs::create_dir_all(&out).unwrap();

        let path = write_migration_bundle(root.path(), &out, "weather", 1)
            .unwrap()
            .expect("bundle path");
        assert!(path.ends_with("migrations.bundle.json"));
        let raw = fs::read_to_string(path).unwrap();
        assert!(raw.contains("\"module_id\": \"weather\""));
        assert!(raw.contains("20260526100000_v1_baseline"));
    }
}
