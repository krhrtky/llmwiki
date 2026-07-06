use serde::Deserialize;
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreContext {
    pub store_id: String,
    pub visibility_store_kind: VisibilityStoreKind,
    pub team_id: Option<String>,
    pub repository_identity: Option<String>,
    pub canonical_root: PathBuf,
}

impl StoreContext {
    pub fn legacy_scope(&self) -> String {
        match self.visibility_store_kind {
            VisibilityStoreKind::Private => "personal",
            VisibilityStoreKind::Team => "team",
            VisibilityStoreKind::Org => "org",
        }
        .to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityStoreKind {
    Private,
    Team,
    Org,
}

impl VisibilityStoreKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Team => "team",
            Self::Org => "org",
        }
    }
}

#[derive(Debug)]
pub enum StorageError {
    Io { message: String },
    InvalidConfig { message: String },
}

impl Display for StorageError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { message } | Self::InvalidConfig { message } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for StorageError {}

#[derive(Debug, Deserialize)]
struct RootConfig {
    storage: StorageConfig,
}

#[derive(Debug, Deserialize)]
struct StorageConfig {
    private: Option<PrivateStoreConfig>,
    #[serde(default)]
    teams: Vec<TeamStoreConfig>,
    org: Option<OrgStoreConfig>,
}

#[derive(Debug, Deserialize)]
struct PrivateStoreConfig {
    path: PathBuf,
    repository: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TeamStoreConfig {
    team_id: String,
    path: PathBuf,
    repository: String,
}

#[derive(Debug, Deserialize)]
struct OrgStoreConfig {
    path: PathBuf,
    repository: String,
}

pub fn resolve_store(config_path: &Path, selector: &str) -> Result<StoreContext, StorageError> {
    let config_path = config_file(config_path)?;
    let config_dir = config_path
        .parent()
        .ok_or_else(|| StorageError::InvalidConfig {
            message: format!("config path has no parent: {}", config_path.display()),
        })?
        .to_path_buf();
    let content = fs::read_to_string(&config_path).map_err(|source| StorageError::Io {
        message: format!(
            "cannot read storage config {}: {source}",
            config_path.display()
        ),
    })?;
    let config: RootConfig =
        serde_yaml::from_str(&content).map_err(|source| StorageError::InvalidConfig {
            message: format!(
                "cannot parse storage config {}: {source}",
                config_path.display()
            ),
        })?;
    validate_storage_config(&config, &config_dir)?;
    resolve_selector(&config, &config_dir, selector)
}

fn config_file(config_path: &Path) -> Result<PathBuf, StorageError> {
    let path = if config_path.is_dir() {
        config_path.join("llmwiki.yaml")
    } else {
        config_path.to_path_buf()
    };
    if !path.is_file() {
        return Err(StorageError::InvalidConfig {
            message: format!("storage config is not a file: {}", path.display()),
        });
    }
    Ok(path)
}

fn resolve_selector(
    config: &RootConfig,
    config_dir: &Path,
    selector: &str,
) -> Result<StoreContext, StorageError> {
    if selector == "private" {
        let Some(private) = &config.storage.private else {
            return Err(StorageError::InvalidConfig {
                message: "private store is not configured".to_string(),
            });
        };
        return Ok(StoreContext {
            store_id: "private".to_string(),
            visibility_store_kind: VisibilityStoreKind::Private,
            team_id: None,
            repository_identity: private.repository.clone(),
            canonical_root: canonical_store_root(config_dir, &private.path)?,
        });
    }

    if selector == "org" {
        let Some(org) = &config.storage.org else {
            return Err(StorageError::InvalidConfig {
                message: "org store is not configured".to_string(),
            });
        };
        return Ok(StoreContext {
            store_id: "org".to_string(),
            visibility_store_kind: VisibilityStoreKind::Org,
            team_id: None,
            repository_identity: Some(org.repository.clone()),
            canonical_root: canonical_store_root(config_dir, &org.path)?,
        });
    }

    let Some(team_id) = selector.strip_prefix("team:") else {
        return Err(StorageError::InvalidConfig {
            message: format!("invalid store selector: {selector}"),
        });
    };
    let team_id = team_id.trim();
    if team_id.is_empty() {
        return Err(StorageError::InvalidConfig {
            message: "team store selector requires team_id".to_string(),
        });
    }
    let Some(team) = config
        .storage
        .teams
        .iter()
        .find(|team| team.team_id == team_id)
    else {
        return Err(StorageError::InvalidConfig {
            message: format!("team store is not configured: {team_id}"),
        });
    };
    Ok(StoreContext {
        store_id: format!("team:{team_id}"),
        visibility_store_kind: VisibilityStoreKind::Team,
        team_id: Some(team.team_id.clone()),
        repository_identity: Some(team.repository.clone()),
        canonical_root: canonical_store_root(config_dir, &team.path)?,
    })
}

fn validate_storage_config(config: &RootConfig, config_dir: &Path) -> Result<(), StorageError> {
    let mut team_ids = BTreeSet::new();
    let mut repositories = BTreeSet::new();
    let mut roots = BTreeSet::new();

    if let Some(private) = &config.storage.private {
        if let Some(repository) = &private.repository {
            insert_unique(&mut repositories, repository, "repository")?;
        }
        insert_unique(
            &mut roots,
            &canonical_store_root(config_dir, &private.path)?
                .display()
                .to_string(),
            "canonical_root",
        )?;
    }

    for team in &config.storage.teams {
        if team.team_id.trim().is_empty() {
            return Err(StorageError::InvalidConfig {
                message: "team_id must not be empty".to_string(),
            });
        }
        insert_unique(&mut team_ids, &team.team_id, "team_id")?;
        insert_unique(&mut repositories, &team.repository, "repository")?;
        insert_unique(
            &mut roots,
            &canonical_store_root(config_dir, &team.path)?
                .display()
                .to_string(),
            "canonical_root",
        )?;
    }

    if let Some(org) = &config.storage.org {
        insert_unique(&mut repositories, &org.repository, "repository")?;
        insert_unique(
            &mut roots,
            &canonical_store_root(config_dir, &org.path)?
                .display()
                .to_string(),
            "canonical_root",
        )?;
    }

    Ok(())
}

fn insert_unique(
    values: &mut BTreeSet<String>,
    value: &str,
    field: &str,
) -> Result<(), StorageError> {
    if !values.insert(value.to_string()) {
        return Err(StorageError::InvalidConfig {
            message: format!("duplicate {field}: {value}"),
        });
    }
    Ok(())
}

fn canonical_store_root(config_dir: &Path, path: &Path) -> Result<PathBuf, StorageError> {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        config_dir.join(path)
    };
    let canonical = fs::canonicalize(&joined).map_err(|source| StorageError::Io {
        message: format!("cannot read store root {}: {source}", joined.display()),
    })?;
    if !canonical.is_dir() {
        return Err(StorageError::InvalidConfig {
            message: format!("store root is not a directory: {}", joined.display()),
        });
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn resolves_team_store_and_rejects_unconfigured_org() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("stores").join("teams").join("platform")).unwrap();
        fs::write(
            dir.path().join("llmwiki.yaml"),
            r#"
storage:
  private:
    path: ./private
  teams:
    - team_id: platform
      repository: git@example.com:platform.git
      path: ./stores/teams/platform
"#,
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("private")).unwrap();

        let store = resolve_store(&dir.path().join("llmwiki.yaml"), "team:platform").unwrap();

        assert_eq!(store.store_id, "team:platform");
        assert_eq!(store.visibility_store_kind, VisibilityStoreKind::Team);
        assert_eq!(store.team_id, Some("platform".to_string()));
        assert_eq!(store.legacy_scope(), "team");

        let error = resolve_store(&dir.path().join("llmwiki.yaml"), "org").unwrap_err();
        assert_eq!(error.to_string(), "org store is not configured");
    }

    #[test]
    fn rejects_duplicate_team_repository() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("a")).unwrap();
        fs::create_dir_all(dir.path().join("b")).unwrap();
        fs::write(
            dir.path().join("llmwiki.yaml"),
            r#"
storage:
  teams:
    - team_id: a
      repository: git@example.com:shared.git
      path: ./a
    - team_id: b
      repository: git@example.com:shared.git
      path: ./b
"#,
        )
        .unwrap();

        let error = resolve_store(&dir.path().join("llmwiki.yaml"), "team:a").unwrap_err();

        assert_eq!(
            error.to_string(),
            "duplicate repository: git@example.com:shared.git"
        );
    }
}
