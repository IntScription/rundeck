use crate::project::Project;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::PathBuf,
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
        let path = config_path()?;

        if !path.exists() {
            let cfg = Self::default();
            cfg.save()?;
            return Ok(cfg);
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;

        Ok(toml::from_str(&content).unwrap_or_default())
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;

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

    pub fn sort_projects(&mut self) {
        self.projects.sort_by(|a, b| {
            b.last_opened
                .unwrap_or_default()
                .cmp(&a.last_opened.unwrap_or_default())
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
