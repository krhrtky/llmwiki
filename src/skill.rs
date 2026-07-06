use crate::report::{InstalledSkill, SkillInstallResult};
use chrono::Utc;
use std::fs;
use std::path::{Component, Path, PathBuf};

const SKILLS_RELATIVE_PATH: &str = "skills";

pub fn install_llmwiki_skill(
    workspace_root: &Path,
    codex_home: Option<PathBuf>,
) -> SkillInstallResult {
    let generated_at = Utc::now().to_rfc3339();
    let source_root = workspace_root.join(SKILLS_RELATIVE_PATH);
    let source_path_string = SKILLS_RELATIVE_PATH.to_string();

    let Some(install_root) = codex_home.or_else(default_codex_home) else {
        return install_error(
            generated_at,
            source_path_string,
            String::new(),
            "codex home is not set",
        );
    };

    let Some(install_root) = valid_absolute_path(install_root) else {
        return install_error(
            generated_at,
            source_path_string,
            String::new(),
            "codex home must be a non-empty absolute path",
        );
    };

    let skill_sources = match list_skill_sources(workspace_root) {
        Ok(sources) if !sources.is_empty() => sources,
        Ok(_) => {
            return install_error(
                generated_at,
                source_path_string,
                install_root
                    .join(SKILLS_RELATIVE_PATH)
                    .display()
                    .to_string(),
                format!("skill source does not exist: {}", source_root.display()),
            );
        }
        Err(message) => {
            return install_error(
                generated_at,
                source_path_string,
                install_root
                    .join(SKILLS_RELATIVE_PATH)
                    .display()
                    .to_string(),
                message,
            );
        }
    };

    if is_existing_symlink(&install_root) {
        return install_error(
            generated_at,
            source_path_string,
            install_root.display().to_string(),
            format!(
                "codex home must not be a symlink: {}",
                install_root.display()
            ),
        );
    }

    if let Err(error) = fs::create_dir_all(&install_root) {
        return install_error(
            generated_at,
            source_path_string,
            install_root.display().to_string(),
            format!(
                "cannot create skill install directory {}: {error}",
                install_root.display()
            ),
        );
    }

    let install_root = match fs::canonicalize(&install_root) {
        Ok(path) => path,
        Err(error) => {
            return install_error(
                generated_at,
                source_path_string,
                install_root.display().to_string(),
                format!("cannot read codex home {}: {error}", install_root.display()),
            );
        }
    };
    let install_path = install_root.join(SKILLS_RELATIVE_PATH);
    let install_path_string = install_path.display().to_string();

    if let Err(message) = reject_existing_symlink_path(&install_path) {
        return install_error(
            generated_at,
            source_path_string,
            install_path_string,
            message,
        );
    }

    let mut installed_skills = Vec::new();
    for skill_source in skill_sources {
        let skill_install_path = install_root
            .join(SKILLS_RELATIVE_PATH)
            .join(&skill_source.name)
            .join("SKILL.md");
        let skill_install_path_string = skill_install_path.display().to_string();

        if let Err(message) = reject_existing_symlink_path(&skill_install_path) {
            return install_error(
                generated_at,
                skill_source.relative_path,
                skill_install_path_string,
                message,
            );
        }

        if let Some(parent) = skill_install_path.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                return install_error(
                    generated_at,
                    skill_source.relative_path,
                    skill_install_path_string,
                    format!(
                        "cannot create skill install directory {}: {error}",
                        parent.display()
                    ),
                );
            }
        }

        if let Err(message) = reject_existing_symlink_path(&skill_install_path) {
            return install_error(
                generated_at,
                skill_source.relative_path,
                skill_install_path_string,
                message,
            );
        }

        if let Err(error) = fs::copy(&skill_source.path, &skill_install_path) {
            return install_error(
                generated_at,
                skill_source.relative_path,
                skill_install_path_string,
                format!(
                    "cannot install skill {} -> {}: {error}",
                    skill_source.path.display(),
                    skill_install_path.display()
                ),
            );
        }

        installed_skills.push(InstalledSkill {
            name: skill_source.name,
            source_path: skill_source.relative_path,
            install_path: skill_install_path_string,
        });
    }

    let installed_count = installed_skills.len();
    SkillInstallResult {
        generated_at,
        status: "success".to_string(),
        skill: "llmwiki-suite".to_string(),
        source_path: source_path_string,
        install_path: install_path_string,
        message: format!("{installed_count} skill(s) installed"),
        installed_skills,
    }
}

struct SkillSource {
    name: String,
    path: PathBuf,
    relative_path: String,
}

fn list_skill_sources(workspace_root: &Path) -> Result<Vec<SkillSource>, String> {
    let skills_root = workspace_root.join(SKILLS_RELATIVE_PATH);
    let entries = fs::read_dir(&skills_root).map_err(|error| {
        format!(
            "cannot read skill source directory {}: {error}",
            skills_root.display()
        )
    })?;
    let mut sources = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "cannot read skill source directory {}: {error}",
                skills_root.display()
            )
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let skill_path = path.join("SKILL.md");
        if !skill_path.is_file() {
            continue;
        }
        sources.push(SkillSource {
            name: name.to_string(),
            path: skill_path,
            relative_path: format!("{SKILLS_RELATIVE_PATH}/{name}/SKILL.md"),
        });
    }

    sources.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(sources)
}

fn install_error(
    generated_at: String,
    source_path: impl Into<String>,
    install_path: impl Into<String>,
    message: impl Into<String>,
) -> SkillInstallResult {
    SkillInstallResult {
        generated_at,
        status: "error".to_string(),
        skill: "llmwiki-suite".to_string(),
        source_path: source_path.into(),
        install_path: install_path.into(),
        message: message.into(),
        installed_skills: Vec::new(),
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
        let skill_path = root
            .join(SKILLS_RELATIVE_PATH)
            .join("llmwiki")
            .join("SKILL.md");
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
