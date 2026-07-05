use crate::report::SkillInstallResult;
use chrono::Utc;
use std::fs;
use std::path::{Component, Path, PathBuf};

const SKILL_RELATIVE_PATH: &str = "skills/llmwiki/SKILL.md";

pub fn install_llmwiki_skill(
    workspace_root: &Path,
    codex_home: Option<PathBuf>,
) -> SkillInstallResult {
    let generated_at = Utc::now().to_rfc3339();
    let source_path = workspace_root.join(SKILL_RELATIVE_PATH);
    let source_path_string = SKILL_RELATIVE_PATH.to_string();

    let Some(install_root) = codex_home.or_else(default_codex_home) else {
        return SkillInstallResult {
            generated_at,
            status: "error".to_string(),
            skill: "llmwiki".to_string(),
            source_path: source_path_string,
            install_path: String::new(),
            message: "codex home is not set".to_string(),
        };
    };

    let Some(install_root) = valid_absolute_path(install_root) else {
        return SkillInstallResult {
            generated_at,
            status: "error".to_string(),
            skill: "llmwiki".to_string(),
            source_path: source_path_string,
            install_path: String::new(),
            message: "codex home must be a non-empty absolute path".to_string(),
        };
    };

    if !source_path.is_file() {
        return SkillInstallResult {
            generated_at,
            status: "error".to_string(),
            skill: "llmwiki".to_string(),
            source_path: source_path_string,
            install_path: install_root.join(SKILL_RELATIVE_PATH).display().to_string(),
            message: format!("skill source does not exist: {}", source_path.display()),
        };
    }

    if is_existing_symlink(&install_root) {
        return SkillInstallResult {
            generated_at,
            status: "error".to_string(),
            skill: "llmwiki".to_string(),
            source_path: source_path_string,
            install_path: install_root.display().to_string(),
            message: format!(
                "codex home must not be a symlink: {}",
                install_root.display()
            ),
        };
    }

    if let Err(error) = fs::create_dir_all(&install_root) {
        return SkillInstallResult {
            generated_at,
            status: "error".to_string(),
            skill: "llmwiki".to_string(),
            source_path: source_path_string,
            install_path: install_root.display().to_string(),
            message: format!(
                "cannot create skill install directory {}: {error}",
                install_root.display()
            ),
        };
    }

    let install_root = match fs::canonicalize(&install_root) {
        Ok(path) => path,
        Err(error) => {
            return SkillInstallResult {
                generated_at,
                status: "error".to_string(),
                skill: "llmwiki".to_string(),
                source_path: source_path_string,
                install_path: install_root.display().to_string(),
                message: format!("cannot read codex home {}: {error}", install_root.display()),
            };
        }
    };
    let install_path = install_root.join(SKILL_RELATIVE_PATH);
    let install_path_string = install_path.display().to_string();

    if let Err(message) = reject_existing_symlink_path(&install_path) {
        return SkillInstallResult {
            generated_at,
            status: "error".to_string(),
            skill: "llmwiki".to_string(),
            source_path: source_path_string,
            install_path: install_path_string,
            message,
        };
    }

    if let Some(parent) = install_path.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            return SkillInstallResult {
                generated_at,
                status: "error".to_string(),
                skill: "llmwiki".to_string(),
                source_path: source_path_string,
                install_path: install_path_string,
                message: format!(
                    "cannot create skill install directory {}: {error}",
                    parent.display()
                ),
            };
        }
    }

    if let Err(message) = reject_existing_symlink_path(&install_path) {
        return SkillInstallResult {
            generated_at,
            status: "error".to_string(),
            skill: "llmwiki".to_string(),
            source_path: source_path_string,
            install_path: install_path_string,
            message,
        };
    }

    if let Err(error) = fs::copy(&source_path, &install_path) {
        return SkillInstallResult {
            generated_at,
            status: "error".to_string(),
            skill: "llmwiki".to_string(),
            source_path: source_path_string,
            install_path: install_path_string,
            message: format!(
                "cannot install skill {} -> {}: {error}",
                source_path.display(),
                install_path.display()
            ),
        };
    }

    SkillInstallResult {
        generated_at,
        status: "success".to_string(),
        skill: "llmwiki".to_string(),
        source_path: source_path_string,
        install_path: install_path_string,
        message: "skill installed".to_string(),
    }
}

fn default_codex_home() -> Option<PathBuf> {
    non_empty_env_path("CODEX_HOME")
        .or_else(|| non_empty_env_path("HOME").map(|home| home.join(".codex")))
}

fn non_empty_env_path(name: &str) -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os(name)?);
    valid_non_empty_path(path)
}

fn valid_non_empty_path(path: PathBuf) -> Option<PathBuf> {
    if path.as_os_str().is_empty() {
        None
    } else {
        Some(path)
    }
}

fn valid_absolute_path(path: PathBuf) -> Option<PathBuf> {
    let path = valid_non_empty_path(path)?;
    if path.is_absolute() {
        Some(path)
    } else {
        None
    }
}

fn is_existing_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
}

fn reject_existing_symlink_path(path: &Path) -> Result<(), String> {
    let mut current = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => current.push(prefix.as_os_str()),
            Component::RootDir => current.push(component.as_os_str()),
            Component::CurDir => continue,
            Component::ParentDir => {
                return Err(format!(
                    "codex home path must not contain parent directory components: {}",
                    path.display()
                ));
            }
            Component::Normal(value) => current.push(value),
        }

        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!(
                    "skill install path must not contain symlink component: {}",
                    current.display()
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "cannot inspect skill install path {}: {error}",
                    current.display()
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_skill(root: &Path) {
        let skill_path = root.join(SKILL_RELATIVE_PATH);
        fs::create_dir_all(skill_path.parent().unwrap()).unwrap();
        fs::write(
            skill_path,
            "---\nname: llmwiki\ndescription: LLMWiki entry point skill\n---\n# LLMWiki\n",
        )
        .unwrap();
    }

    #[test]
    fn install_rejects_relative_codex_home() {
        let workspace = tempdir().unwrap();
        write_skill(workspace.path());

        let result = install_llmwiki_skill(workspace.path(), Some(PathBuf::from("relative")));

        assert_eq!(result.status, "error");
        assert_eq!(
            result.message,
            "codex home must be a non-empty absolute path"
        );
    }

    #[cfg(unix)]
    #[test]
    fn install_rejects_symlink_component() {
        use std::os::unix::fs::symlink;

        let workspace = tempdir().unwrap();
        write_skill(workspace.path());
        let codex_home = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let skills_path = codex_home.path().join("skills");
        symlink(outside.path(), &skills_path).unwrap();

        let result = install_llmwiki_skill(workspace.path(), Some(codex_home.path().to_path_buf()));

        assert_eq!(result.status, "error");
        assert!(result.message.contains("symlink component"));
        assert!(!outside.path().join("llmwiki").join("SKILL.md").exists());
    }
}
