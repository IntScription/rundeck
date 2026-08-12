use crate::project::Project;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_editor")]
    pub editor: String,

    #[serde(default = "default_shell")]
    pub shell: String,

    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default = "default_top_pane_ratio")]
    pub top_pane_ratio: u16,

    #[serde(default = "default_show_icons")]
    pub show_icons: bool,

    #[serde(default = "default_project_picker")]
    pub project_picker: String,

    #[serde(default)]
    pub project_roots: Vec<String>,

    #[serde(default)]
    pub keymaps: Keymaps,

    #[serde(default)]
    pub rundeck_session: Option<String>,

    #[serde(default)]
    pub projects: Vec<Project>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keymaps {
    #[serde(default = "key_quit")]
    pub quit: String,

    #[serde(default = "key_help")]
    pub help: String,

    #[serde(default = "key_search")]
    pub search: String,

    #[serde(default = "key_add_project")]
    pub add_project: String,

    #[serde(default = "key_create_project")]
    pub create_project: String,

    #[serde(default = "key_remove_project")]
    pub remove_project: String,

    #[serde(default = "key_workspace")]
    pub workspace: String,

    #[serde(default = "key_workspace_alt")]
    pub workspace_alt: String,

    #[serde(default = "key_local_preview")]
    pub local_preview: String,

    #[serde(default = "key_deploy_preview")]
    pub deploy_preview: String,

    #[serde(default = "key_editor")]
    pub editor: String,

    #[serde(default = "key_lazygit")]
    pub lazygit: String,

    #[serde(default = "key_edit_deploy")]
    pub edit_deploy: String,

    #[serde(default = "key_config")]
    pub config: String,

    #[serde(default = "key_theme")]
    pub theme: String,

    #[serde(default = "key_doctor")]
    pub doctor: String,

    #[serde(default = "key_kill_session")]
    pub kill_session: String,

    #[serde(default = "key_stop_dev")]
    pub stop_dev: String,

    #[serde(default = "key_reload")]
    pub reload: String,

    #[serde(default = "key_pin")]
    pub pin: String,

    #[serde(default = "key_left")]
    pub left: String,

    #[serde(default = "key_right")]
    pub right: String,

    #[serde(default = "key_down")]
    pub down: String,

    #[serde(default = "key_up")]
    pub up: String,
}

impl Default for Keymaps {
    fn default() -> Self {
        Self {
            quit: key_quit(),
            help: key_help(),
            search: key_search(),
            add_project: key_add_project(),
            create_project: key_create_project(),
            remove_project: key_remove_project(),
            workspace: key_workspace(),
            workspace_alt: key_workspace_alt(),
            local_preview: key_local_preview(),
            deploy_preview: key_deploy_preview(),
            editor: key_editor(),
            lazygit: key_lazygit(),
            edit_deploy: key_edit_deploy(),
            config: key_config(),
            theme: key_theme(),
            doctor: key_doctor(),
            kill_session: key_kill_session(),
            stop_dev: key_stop_dev(),
            reload: key_reload(),
            pin: key_pin(),
            left: key_left(),
            right: key_right(),
            down: key_down(),
            up: key_up(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            editor: default_editor(),
            shell: default_shell(),
            theme: default_theme(),
            top_pane_ratio: default_top_pane_ratio(),
            show_icons: default_show_icons(),
            project_picker: default_project_picker(),
            project_roots: Vec::new(),
            keymaps: Keymaps::default(),
            rundeck_session: None,
            projects: Vec::new(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        Self::load_from(&config_path()?)
    }

    fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            let cfg = Self::default();
            cfg.save_to(path)?;
            return Ok(cfg);
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;

        toml::from_str(&content).with_context(|| {
            format!(
                "Failed to parse config at {}. Fix the file, or restore it from {}.bak if one exists.",
                path.display(),
                path.display()
            )
        })
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&config_path()?)
    }

    /// Writes via a temp file + rename so a crash mid-write can't truncate
    /// the config, and keeps a `.bak` of whatever was previously on disk so
    /// a corrupted or bad edit can be recovered from.
    fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        let file_name = path
            .file_name()
            .context("Config path has no file name")?
            .to_string_lossy()
            .to_string();

        if path.exists() {
            let backup_path = path.with_file_name(format!("{file_name}.bak"));
            fs::copy(path, &backup_path).with_context(|| {
                format!("Failed to back up config to {}", backup_path.display())
            })?;
        }

        let tmp_path = path.with_file_name(format!("{file_name}.tmp"));
        fs::write(&tmp_path, content)
            .with_context(|| format!("Failed to write temp config at {}", tmp_path.display()))?;
        fs::rename(&tmp_path, path)
            .with_context(|| format!("Failed to save config at {}", path.display()))?;

        Ok(())
    }

    pub fn add_project(
        &mut self,
        path: PathBuf,
        name: Option<String>,
        port: Option<u16>,
        deploy_url: Option<String>,
    ) -> Result<String> {
        let absolute_path = path
            .canonicalize()
            .with_context(|| format!("Project path does not exist: {}", path.display()))?;

        let project_name = name.unwrap_or_else(|| {
            absolute_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

        if let Some(existing) = self
            .projects
            .iter_mut()
            .find(|project| project.name == project_name || project.path == absolute_path)
        {
            existing.name = project_name.clone();
            existing.path = absolute_path;
            existing.port = port.or(existing.port);
            existing.deploy_url = deploy_url.or_else(|| existing.deploy_url.clone());
            return Ok(project_name);
        }

        self.projects.push(Project {
            name: project_name.clone(),
            path: absolute_path,
            port,
            deploy_url,
            dev_command: None,
            last_opened: None,
            pinned: false,
        });

        Ok(project_name)
    }

    pub fn remove_project(&mut self, name: &str) -> bool {
        let before = self.projects.len();

        self.projects.retain(|project| {
            project.name != name && project.tmux_session_name() != Project::slug_name(name)
        });

        self.projects.len() != before
    }

    pub fn prune_missing_projects(&mut self) -> usize {
        let before = self.projects.len();

        self.projects.retain(|project| project.path.exists());

        before.saturating_sub(self.projects.len())
    }

    pub fn project_by_name(&self, name: &str) -> Option<&Project> {
        self.projects
            .iter()
            .find(|project| project.name == name || project.tmux_session_name() == name)
    }

    pub fn touch_project(&mut self, name: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default();

        if let Some(project) = self
            .projects
            .iter_mut()
            .find(|project| project.name == name || project.tmux_session_name() == name)
        {
            project.last_opened = Some(now);
        }
    }

    /// Flips the pinned state of a project and returns the new value.
    pub fn toggle_pinned(&mut self, name: &str) -> bool {
        let Some(project) = self
            .projects
            .iter_mut()
            .find(|project| project.name == name || project.tmux_session_name() == name)
        else {
            return false;
        };

        project.pinned = !project.pinned;
        project.pinned
    }

    pub fn sort_projects(&mut self) {
        self.projects.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then_with(|| {
                    b.last_opened
                        .unwrap_or_default()
                        .cmp(&a.last_opened.unwrap_or_default())
                })
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
    }

    pub fn project_picker_enabled(&self) -> bool {
        let value = self.project_picker.trim().to_lowercase();

        !matches!(value.as_str(), "" | "none" | "off" | "false" | "disabled")
    }
}

pub fn config_path() -> Result<PathBuf> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
        .context("Could not find home directory")?;

    Ok(home.join(".config").join("rundeck").join("config.toml"))
}

fn default_editor() -> String {
    "nvim".to_string()
}

fn default_shell() -> String {
    env::var("SHELL").unwrap_or_else(|_| "zsh".to_string())
}

fn default_theme() -> String {
    "catppuccin-mocha".to_string()
}

fn default_top_pane_ratio() -> u16 {
    70
}

fn default_show_icons() -> bool {
    true
}

fn default_project_picker() -> String {
    "fzf".to_string()
}

fn key_quit() -> String {
    "q".to_string()
}

fn key_help() -> String {
    "?".to_string()
}

fn key_search() -> String {
    "/".to_string()
}

fn key_add_project() -> String {
    "a".to_string()
}

fn key_create_project() -> String {
    "c".to_string()
}

fn key_remove_project() -> String {
    "d".to_string()
}

fn key_workspace() -> String {
    "enter".to_string()
}

fn key_workspace_alt() -> String {
    "t".to_string()
}

fn key_local_preview() -> String {
    "b".to_string()
}

fn key_deploy_preview() -> String {
    "B".to_string()
}

fn key_editor() -> String {
    "o".to_string()
}

fn key_lazygit() -> String {
    "g".to_string()
}

fn key_edit_deploy() -> String {
    "u".to_string()
}

fn key_config() -> String {
    "e".to_string()
}

fn key_theme() -> String {
    "T".to_string()
}

fn key_doctor() -> String {
    "D".to_string()
}

fn key_kill_session() -> String {
    "x".to_string()
}

fn key_stop_dev() -> String {
    "X".to_string()
}

fn key_reload() -> String {
    "r".to_string()
}

fn key_pin() -> String {
    "p".to_string()
}

fn key_left() -> String {
    "h".to_string()
}

fn key_right() -> String {
    "l".to_string()
}

fn key_down() -> String {
    "j".to_string()
}

fn key_up() -> String {
    "k".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn config_file_path(dir: &TempDir) -> PathBuf {
        dir.path().join("config.toml")
    }

    #[test]
    fn load_from_missing_path_creates_default_and_persists_it() {
        let dir = TempDir::new().unwrap();
        let path = config_file_path(&dir);
        assert!(!path.exists());

        let cfg = Config::load_from(&path).unwrap();

        assert_eq!(cfg.theme, default_theme());
        assert!(path.exists(), "load_from should save the default config");
    }

    #[test]
    fn save_then_load_roundtrips_projects() {
        let dir = TempDir::new().unwrap();
        let path = config_file_path(&dir);

        let mut cfg = Config::default();
        cfg.projects.push(Project {
            name: "My App".to_string(),
            path: PathBuf::from("/tmp/my-app"),
            port: Some(3000),
            deploy_url: Some("https://example.com".to_string()),
            dev_command: None,
            last_opened: Some(42),
            pinned: false,
        });

        cfg.save_to(&path).unwrap();
        let reloaded = Config::load_from(&path).unwrap();

        assert_eq!(reloaded.projects.len(), 1);
        assert_eq!(reloaded.projects[0].name, "My App");
        assert_eq!(reloaded.projects[0].port, Some(3000));
    }

    #[test]
    fn load_from_malformed_toml_errors_instead_of_silently_resetting() {
        let dir = TempDir::new().unwrap();
        let path = config_file_path(&dir);
        fs::write(&path, "this is not [valid toml").unwrap();

        let result = Config::load_from(&path);

        assert!(
            result.is_err(),
            "a malformed config must error, not silently return an empty default \
             (that would wipe the user's projects on the next save)"
        );
    }

    #[test]
    fn save_to_keeps_backup_of_previous_version() {
        let dir = TempDir::new().unwrap();
        let path = config_file_path(&dir);

        let first = Config {
            theme: "nord".to_string(),
            ..Config::default()
        };
        first.save_to(&path).unwrap();

        let mut second = first.clone();
        second.theme = "dracula".to_string();
        second.save_to(&path).unwrap();

        let backup_path = path.with_file_name("config.toml.bak");
        assert!(backup_path.exists());

        let backup_content = fs::read_to_string(&backup_path).unwrap();
        assert!(backup_content.contains("nord"));

        let current_content = fs::read_to_string(&path).unwrap();
        assert!(current_content.contains("dracula"));
    }

    fn project_with_path(name: &str, path: PathBuf) -> Project {
        Project {
            name: name.to_string(),
            path,
            port: None,
            deploy_url: None,
            dev_command: None,
            last_opened: None,
            pinned: false,
        }
    }

    #[test]
    fn add_project_updates_existing_entry_matched_by_path() {
        let dir = TempDir::new().unwrap();
        let project_dir = dir.path().join("my-app");
        fs::create_dir_all(&project_dir).unwrap();

        let mut cfg = Config::default();
        let name = cfg
            .add_project(project_dir.clone(), None, Some(3000), None)
            .unwrap();
        assert_eq!(name, "my-app");
        assert_eq!(cfg.projects.len(), 1);

        // Re-adding the same path with a new port should update, not duplicate.
        cfg.add_project(project_dir.clone(), None, Some(4000), None)
            .unwrap();

        assert_eq!(cfg.projects.len(), 1);
        assert_eq!(cfg.projects[0].port, Some(4000));
    }

    #[test]
    fn remove_project_matches_by_name_or_slug() {
        let mut cfg = Config::default();
        cfg.projects
            .push(project_with_path("My App", PathBuf::from("/tmp/my-app")));

        assert!(cfg.remove_project("my-app"));
        assert!(cfg.projects.is_empty());
    }

    #[test]
    fn prune_missing_projects_drops_paths_that_no_longer_exist() {
        let dir = TempDir::new().unwrap();
        let existing = dir.path().join("exists");
        fs::create_dir_all(&existing).unwrap();
        let missing = dir.path().join("gone");

        let mut cfg = Config::default();
        cfg.projects.push(project_with_path("exists", existing));
        cfg.projects.push(project_with_path("gone", missing));

        let pruned = cfg.prune_missing_projects();

        assert_eq!(pruned, 1);
        assert_eq!(cfg.projects.len(), 1);
        assert_eq!(cfg.projects[0].name, "exists");
    }

    #[test]
    fn sort_projects_orders_by_recency_then_name() {
        let mut cfg = Config::default();
        cfg.projects.push(Project {
            last_opened: Some(10),
            ..project_with_path("Older", PathBuf::from("/tmp/older"))
        });
        cfg.projects.push(Project {
            last_opened: Some(20),
            ..project_with_path("Newer", PathBuf::from("/tmp/newer"))
        });
        cfg.projects.push(project_with_path(
            "Never Opened",
            PathBuf::from("/tmp/never"),
        ));

        cfg.sort_projects();

        let names: Vec<_> = cfg.projects.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Newer", "Older", "Never Opened"]);
    }

    #[test]
    fn sort_projects_puts_pinned_projects_first_regardless_of_recency() {
        let mut cfg = Config::default();
        cfg.projects.push(Project {
            last_opened: Some(100),
            ..project_with_path("Recent", PathBuf::from("/tmp/recent"))
        });
        cfg.projects.push(Project {
            last_opened: Some(1),
            pinned: true,
            ..project_with_path("Pinned But Old", PathBuf::from("/tmp/pinned"))
        });

        cfg.sort_projects();

        let names: Vec<_> = cfg.projects.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Pinned But Old", "Recent"]);
    }

    #[test]
    fn toggle_pinned_flips_state_and_returns_new_value() {
        let mut cfg = Config::default();
        cfg.projects
            .push(project_with_path("My App", PathBuf::from("/tmp/my-app")));

        assert!(cfg.toggle_pinned("my-app"));
        assert!(cfg.projects[0].pinned);

        assert!(!cfg.toggle_pinned("My App"));
        assert!(!cfg.projects[0].pinned);
    }

    #[test]
    fn project_picker_enabled_treats_disabled_values_as_off() {
        let mut cfg = Config::default();
        assert!(cfg.project_picker_enabled());

        for value in ["none", "off", "false", "disabled", "", "  "] {
            cfg.project_picker = value.to_string();
            assert!(
                !cfg.project_picker_enabled(),
                "expected {value:?} to disable the picker"
            );
        }
    }
}
