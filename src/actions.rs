use crate::{
    config::{self, Config},
    project::{Project, slugify},
};
use anyhow::{Context, Result};
use std::{
    collections::BTreeSet,
    env, fs,
    io::Write,
    net::TcpStream,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub fn is_inside_tmux() -> bool {
    env::var("TMUX").is_ok()
}

pub fn open_dashboard_session() -> Result<()> {
    if !command_exists("tmux") {
        anyhow::bail!("tmux is not installed. Install it with: brew install tmux");
    }

    let exe = env::current_exe().context("Failed to get RunDeck executable path")?;
    let exe = exe.to_string_lossy().to_string();

    Command::new("tmux")
        .args(["new-session", "-A", "-s", "rundeck-dashboard"])
        .arg(exe)
        .arg("--dashboard")
        .status()
        .context("Failed to open RunDeck dashboard tmux session")?;

    Ok(())
}

pub fn remember_current_rundeck_session(cfg: &mut Config) {
    if env::var("TMUX").is_err() {
        return;
    }

    if let Ok(Some(session)) = current_tmux_session() {
        cfg.rundeck_session = Some(session);
    }
}

pub fn doctor(cfg: &Config) {
    println!("RunDeck Doctor\n");

    for item in doctor_items(cfg) {
        let status = if item.ok { "✓" } else { "✗" };
        println!("{status} {} — {}", item.name, item.note);
    }

    println!("\nConfig:");
    println!("  {}", config::config_path().unwrap().display());

    println!("\nIf something is missing on macOS:");
    println!("  brew install tmux lazygit git fzf");
}

#[derive(Debug, Clone)]
pub struct DoctorItem {
    pub name: String,
    pub ok: bool,
    pub note: String,
}

pub fn doctor_items(cfg: &Config) -> Vec<DoctorItem> {
    let mut items = vec![
        doctor_item("git", "required for repositories"),
        doctor_item("tmux", "required for workspaces"),
        doctor_item("lazygit", "used by g shortcut"),
        doctor_item("fzf", "used by add/create project pickers"),
        doctor_item(&cfg.editor, "configured editor"),
        DoctorItem {
            name: "~/.cargo/bin in PATH".to_string(),
            ok: env::var("PATH")
                .map(|path| path.split(':').any(|part| part.ends_with(".cargo/bin")))
                .unwrap_or(false),
            note: "needed for rundeck back/close/kill from any tmux pane".to_string(),
        },
    ];

    if !cfg.project_picker_enabled() {
        items.push(DoctorItem {
            name: "project picker".to_string(),
            ok: true,
            note: "disabled in config".to_string(),
        });
    }

    items
}

fn doctor_item(binary: &str, note: &str) -> DoctorItem {
    DoctorItem {
        name: binary.to_string(),
        ok: command_exists(binary),
        note: note.to_string(),
    }
}

fn command_exists(binary: &str) -> bool {
    #[cfg(windows)]
    {
        Command::new("where")
            .arg(binary)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[cfg(not(windows))]
    {
        Command::new("sh")
            .arg("-lc")
            .arg(format!("command -v {binary}"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
}

pub fn add_project_with_picker(cfg: &mut Config) -> Result<Option<String>> {
    if !cfg.project_picker_enabled() {
        anyhow::bail!("Project picker is disabled in config.");
    }

    if cfg.project_picker.trim() != "fzf" {
        anyhow::bail!(
            "Unsupported project picker: {}. Currently supported: fzf",
            cfg.project_picker
        );
    }

    if !command_exists("fzf") {
        anyhow::bail!("fzf is not installed. Install it with: brew install fzf");
    }

    let Some(path) = pick_existing_project_with_fzf(cfg)? else {
        return Ok(None);
    };

    let port = detect_project_port(&path);
    let name = cfg.add_project(path, None, port, None)?;

    cfg.touch_project(&name);
    cfg.sort_projects();
    cfg.save()?;

    Ok(Some(name))
}

pub fn create_project_with_picker(cfg: &mut Config, folder_name: &str) -> Result<Option<String>> {
    let folder_name = folder_name.trim();

    if folder_name.is_empty() {
        anyhow::bail!("Project folder name cannot be empty.");
    }

    if folder_name.contains('/') || folder_name.contains('\\') {
        anyhow::bail!("Use only the folder name, not a path.");
    }

    if !cfg.project_picker_enabled() {
        anyhow::bail!("Project picker is disabled in config.");
    }

    if cfg.project_picker.trim() != "fzf" {
        anyhow::bail!(
            "Unsupported project picker: {}. Currently supported: fzf",
            cfg.project_picker
        );
    }

    if !command_exists("fzf") {
        anyhow::bail!("fzf is not installed. Install it with: brew install fzf");
    }

    let Some(parent) = pick_parent_dir_with_fzf(cfg)? else {
        return Ok(None);
    };

    let project_path = parent.join(folder_name);

    if project_path.exists() && !project_path.is_dir() {
        anyhow::bail!("A file already exists at {}", project_path.display());
    }

    fs::create_dir_all(&project_path)
        .with_context(|| format!("Failed to create {}", project_path.display()))?;

    let name = cfg.add_project(
        project_path.clone(),
        Some(folder_name.to_string()),
        detect_project_port(&project_path),
        None,
    )?;

    cfg.touch_project(&name);
    cfg.sort_projects();
    cfg.save()?;

    let project = cfg
        .project_by_name(&name)
        .cloned()
        .with_context(|| format!("Created project but could not reload it: {name}"))?;

    open_workspace(cfg, &project)?;

    Ok(Some(name))
}

fn pick_existing_project_with_fzf(cfg: &Config) -> Result<Option<PathBuf>> {
    let roots = project_roots(cfg);
    let candidates = discover_project_dirs(&roots);

    if candidates.is_empty() {
        anyhow::bail!(
            "No projects found. Add roots in config.toml with project_roots = [\"~/Projects\", \"~/Work\"]"
        );
    }

    pick_path_with_fzf("Add project > ", &candidates)
}

fn pick_parent_dir_with_fzf(cfg: &Config) -> Result<Option<PathBuf>> {
    let roots = project_roots(cfg);
    let candidates = discover_parent_dirs(&roots);

    if candidates.is_empty() {
        anyhow::bail!(
            "No folders found. Add roots in config.toml with project_roots = [\"~/Projects\", \"~/Work\"]"
        );
    }

    pick_path_with_fzf("Create in > ", &candidates)
}

fn pick_path_with_fzf(prompt: &str, candidates: &[PathBuf]) -> Result<Option<PathBuf>> {
    let input = candidates
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join("\n");

    let mut child = Command::new("fzf")
        .args([
            "--prompt", prompt, "--height", "85%", "--layout", "reverse", "--border", "--ansi",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to start fzf")?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(input.as_bytes())?;
    }

    let output = child.wait_with_output()?;

    if !output.status.success() {
        return Ok(None);
    }

    let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if selected.is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(selected)))
    }
}

fn project_roots(cfg: &Config) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if !cfg.project_roots.is_empty() {
        for root in &cfg.project_roots {
            let path = expand_home(root);

            if path.is_dir() {
                roots.push(path);
            }
        }

        return dedupe_paths(roots);
    }

    let Some(home) = home_dir() else {
        if let Ok(current) = env::current_dir() {
            roots.push(current);
        }

        return dedupe_paths(roots);
    };

    let common = [
        "Projects",
        "Developer",
        "Code",
        "code",
        "dev",
        "workspace",
        "repos",
        "src",
        "Documents/GitHub",
    ];

    for item in common {
        let path = home.join(item);

        if path.is_dir() {
            roots.push(path);
        }
    }

    if roots.is_empty() {
        roots.push(home);
    }

    if let Ok(current) = env::current_dir() {
        roots.push(current);
    }

    dedupe_paths(roots)
}

fn discover_project_dirs(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut found = BTreeSet::new();

    for root in roots {
        collect_project_dirs(root, 4, &mut found);
    }

    found.into_iter().collect()
}

fn discover_parent_dirs(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut found = BTreeSet::new();

    for root in roots {
        collect_parent_dirs(root, 4, &mut found);
    }

    found.into_iter().collect()
}

fn collect_project_dirs(path: &Path, depth: usize, found: &mut BTreeSet<PathBuf>) {
    if !path.is_dir() || should_ignore_dir(path) {
        return;
    }

    // Important:
    // If this directory is a repo/workspace root, add it as ONE RunDeck project
    // and do not recurse into web/mobile/apps packages.
    //
    // Example:
    // rest-assured/
    // ├─ web/package.json
    // ├─ mobile/package.json
    // └─ supabase/
    //
    // This should show only "rest-assured", not "web" and "mobile" separately.
    if is_repo_root(path) || is_workspace_root(path) {
        found.insert(path.to_path_buf());
        return;
    }

    // Standalone project/package.
    if is_project_dir(path) {
        found.insert(path.to_path_buf());
        return;
    }

    if depth == 0 {
        return;
    }

    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let child = entry.path();

        if child.is_dir() {
            collect_project_dirs(&child, depth - 1, found);
        }
    }
}

fn collect_parent_dirs(path: &Path, depth: usize, found: &mut BTreeSet<PathBuf>) {
    if !path.is_dir() || should_ignore_dir(path) {
        return;
    }

    found.insert(path.to_path_buf());

    if depth == 0 {
        return;
    }

    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let child = entry.path();

        if child.is_dir() {
            collect_parent_dirs(&child, depth - 1, found);
        }
    }
}

fn is_repo_root(path: &Path) -> bool {
    path.join(".git").exists()
}

fn is_workspace_root(path: &Path) -> bool {
    let app_dirs = [
        "web",
        "mobile",
        "frontend",
        "client",
        "site",
        "desktop",
        "server",
        "api",
        "backend",
        "apps/web",
        "apps/mobile",
        "apps/app",
        "apps/api",
        "packages/web",
        "packages/mobile",
        "packages/ui",
    ];

    let app_count = app_dirs
        .iter()
        .filter(|dir| {
            let dir_path = path.join(dir);

            dir_path.join("package.json").exists()
                || dir_path.join("Cargo.toml").exists()
                || dir_path.join("pubspec.yaml").exists()
                || dir_path.join("pyproject.toml").exists()
                || dir_path.join("go.mod").exists()
        })
        .count();

    if app_count >= 2 {
        return true;
    }

    let has_frontend = ["web", "frontend", "client", "site", "apps/web"]
        .iter()
        .any(|dir| path.join(dir).join("package.json").exists());

    let has_mobile = ["mobile", "apps/mobile", "apps/app"]
        .iter()
        .any(|dir| path.join(dir).join("package.json").exists());

    let has_backend_or_infra = path.join("supabase").exists()
        || path.join("server").join("package.json").exists()
        || path.join("api").join("package.json").exists()
        || path.join("backend").join("package.json").exists()
        || path.join("apps").join("api").join("package.json").exists();

    (has_frontend || has_mobile) && has_backend_or_infra
}

fn is_project_dir(path: &Path) -> bool {
    let markers = [
        ".git",
        "package.json",
        "Cargo.toml",
        "go.mod",
        "pyproject.toml",
        "composer.json",
        "pubspec.yaml",
        "pom.xml",
        "build.gradle",
        "deno.json",
        "bun.lock",
        "pnpm-lock.yaml",
        "yarn.lock",
    ];

    markers.iter().any(|marker| path.join(marker).exists())
}

fn should_ignore_dir(path: &Path) -> bool {
    let ignored = [
        ".git",
        "node_modules",
        ".next",
        "dist",
        "build",
        "target",
        ".vercel",
        ".turbo",
        ".expo",
        ".cache",
        "Library",
        "Applications",
    ];

    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| ignored.contains(&name))
        .unwrap_or(false)
}

fn detect_project_port(path: &Path) -> Option<u16> {
    let package_json = path.join("package.json");

    if package_json.exists()
        && let Ok(content) = fs::read_to_string(&package_json)
    {
        return detect_port_from_package_text(&content);
    }

    let common_package_dirs = [
        "web",
        "frontend",
        "client",
        "site",
        "apps/web",
        "packages/web",
    ];

    for dir in common_package_dirs {
        let nested = path.join(dir).join("package.json");

        if !nested.exists() {
            continue;
        }

        if let Ok(content) = fs::read_to_string(nested) {
            return detect_port_from_package_text(&content);
        }
    }

    None
}

fn detect_port_from_package_text(content: &str) -> Option<u16> {
    if let Some(port) = extract_port_from_text(content) {
        return Some(port);
    }

    let lower = content.to_lowercase();

    if lower.contains("\"next\"") {
        Some(3000)
    } else if lower.contains("\"vite\"") {
        Some(5173)
    } else if lower.contains("\"expo\"") {
        Some(8081)
    } else {
        None
    }
}

fn extract_port_from_text(text: &str) -> Option<u16> {
    let markers = ["localhost:", "127.0.0.1:", "--port", "-p", "PORT="];

    for marker in markers {
        if let Some(port) = extract_after_marker(text, marker) {
            return Some(port);
        }
    }

    None
}

fn extract_after_marker(text: &str, marker: &str) -> Option<u16> {
    let index = text.find(marker)?;
    let after = &text[index + marker.len()..];

    let digits: String = after
        .chars()
        .skip_while(|ch| ch.is_whitespace() || *ch == '=' || *ch == ':' || *ch == '"')
        .take_while(|ch| ch.is_ascii_digit())
        .collect();

    if digits.is_empty() {
        None
    } else {
        digits.parse::<u16>().ok()
    }
}

fn expand_home(value: &str) -> PathBuf {
    if value == "~" {
        return home_dir().unwrap_or_else(|| PathBuf::from(value));
    }

    if let Some(rest) = value.strip_prefix("~/")
        && let Some(home) = home_dir()
    {
        return home.join(rest);
    }

    PathBuf::from(value)
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = BTreeSet::new();
    let mut output = Vec::new();

    for path in paths {
        let normalized = path.canonicalize().unwrap_or(path);

        if seen.insert(normalized.clone()) {
            output.push(normalized);
        }
    }

    output
}

pub fn open_workspace(cfg: &Config, project: &Project) -> Result<()> {
    if !command_exists("tmux") {
        anyhow::bail!("tmux is not installed. Install it with: brew install tmux");
    }

    let session = project.tmux_session_name();

    if !tmux_session_exists(&session)? {
        create_tmux_workspace(cfg, project, &session)?;
    } else {
        ensure_bottom_pane(cfg, project, &session)?;
    }

    attach_or_switch_tmux(&session)?;

    Ok(())
}

fn tmux_session_exists(session: &str) -> Result<bool> {
    let status = Command::new("tmux")
        .args(["has-session", "-t", session])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("Failed to check tmux session")?;

    Ok(status.success())
}

fn create_tmux_workspace(cfg: &Config, project: &Project, session: &str) -> Result<()> {
    let path = project.path.to_string_lossy().to_string();
    let editor_command = format!("{} .", cfg.editor);

    Command::new("tmux")
        .args(["new-session", "-d", "-s", session, "-c", path.as_str()])
        .arg(editor_command)
        .status()
        .context("Failed to create tmux session")?;

    ensure_bottom_pane(cfg, project, session)?;

    let top_pane = pane_id(session, 0)?;

    Command::new("tmux")
        .args(["select-pane", "-t", top_pane.as_str()])
        .status()
        .context("Failed to select tmux top pane")?;

    Ok(())
}

fn ensure_bottom_pane(cfg: &Config, project: &Project, session: &str) -> Result<()> {
    let panes = pane_ids(session)?;

    if panes.len() >= 2 {
        return Ok(());
    }

    let path = project.path.to_string_lossy().to_string();

    let bottom_percent = 100u16
        .saturating_sub(cfg.top_pane_ratio)
        .clamp(10, 90)
        .to_string();

    let bottom_command = format!(
        "printf '\\nRunDeck commands:\\n  rundeck back   # return to dashboard\\n  rundeck close  # return and close this project session\\n  rundeck kill   # kill current tmux session\\n\\n'; exec {} -l",
        cfg.shell
    );

    Command::new("tmux")
        .args([
            "split-window",
            "-v",
            "-p",
            bottom_percent.as_str(),
            "-t",
            session,
            "-c",
            path.as_str(),
        ])
        .arg(cfg.shell.as_str())
        .arg("-lc")
        .arg(bottom_command)
        .status()
        .context("Failed to split tmux window")?;

    Ok(())
}

fn pane_ids(session: &str) -> Result<Vec<String>> {
    let output = Command::new("tmux")
        .args(["list-panes", "-t", session, "-F", "#{pane_id}"])
        .output()
        .with_context(|| format!("Failed to list panes for tmux session: {session}"))?;

    if !output.status.success() {
        anyhow::bail!("Failed to list panes for tmux session: {session}");
    }

    let panes = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect();

    Ok(panes)
}

fn pane_id(session: &str, index: usize) -> Result<String> {
    let panes = pane_ids(session)?;

    panes
        .get(index)
        .cloned()
        .with_context(|| format!("Could not find pane index {index} in tmux session: {session}"))
}

fn attach_or_switch_tmux(session: &str) -> Result<()> {
    if env::var("TMUX").is_ok() {
        Command::new("tmux")
            .args(["switch-client", "-t", session])
            .status()
            .context("Failed to switch tmux session")?;
    } else {
        Command::new("tmux")
            .args(["attach-session", "-t", session])
            .status()
            .context("Failed to attach tmux session")?;
    }

    Ok(())
}

pub fn back_to_rundeck(cfg: &Config) -> Result<()> {
    let Some(session) = &cfg.rundeck_session else {
        anyhow::bail!("No RunDeck session remembered yet. Open RunDeck from tmux first.");
    };

    switch_tmux_session(session)
}

pub fn close_current_session(cfg: &Config) -> Result<()> {
    let current = current_tmux_session()?.context("Not currently inside tmux")?;

    if let Some(rundeck_session) = &cfg.rundeck_session
        && &current != rundeck_session
        && tmux_session_exists(rundeck_session)?
    {
        switch_tmux_session(rundeck_session)?;
    }

    kill_tmux_session(&current)?;

    Ok(())
}

pub fn kill_session_command(cfg: &Config, name: Option<String>) -> Result<()> {
    let session = match name {
        Some(name) => cfg
            .project_by_name(&name)
            .map(Project::tmux_session_name)
            .unwrap_or_else(|| slugify(&name)),
        None => current_tmux_session()?.context("Not currently inside tmux")?,
    };

    kill_tmux_session(&session)
}

pub fn kill_project_session(project: &Project) -> Result<()> {
    let session = project.tmux_session_name();

    if tmux_session_exists(&session)? {
        kill_tmux_session(&session)?;
    }

    Ok(())
}

pub fn stop_project_dev(project: &Project) -> Result<()> {
    let session = project.tmux_session_name();

    if !tmux_session_exists(&session)? {
        return Ok(());
    }

    let Ok(bottom_pane) = pane_id(&session, 1) else {
        return Ok(());
    };

    Command::new("tmux")
        .args(["send-keys", "-t", bottom_pane.as_str(), "C-c"])
        .status()
        .context("Failed to stop dev server")?;

    Ok(())
}

pub fn dev_server_status(project: &Project) -> String {
    let session = project.tmux_session_name();

    let Ok(true) = tmux_session_exists(&session) else {
        return "Stopped".to_string();
    };

    let Ok(bottom_pane) = pane_id(&session, 1) else {
        return "Stopped".to_string();
    };

    match pane_has_running_process(&bottom_pane) {
        Ok(true) => "Running".to_string(),
        Ok(false) => "Stopped".to_string(),
        Err(_) => "Unknown".to_string(),
    }
}

fn current_tmux_session() -> Result<Option<String>> {
    if env::var("TMUX").is_err() {
        return Ok(None);
    }

    let output = Command::new("tmux")
        .args(["display-message", "-p", "#S"])
        .output()
        .context("Failed to get current tmux session")?;

    if !output.status.success() {
        return Ok(None);
    }

    let session = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if session.is_empty() {
        Ok(None)
    } else {
        Ok(Some(session))
    }
}

fn switch_tmux_session(session: &str) -> Result<()> {
    Command::new("tmux")
        .args(["switch-client", "-t", session])
        .status()
        .with_context(|| format!("Failed to switch to tmux session: {session}"))?;

    Ok(())
}

fn kill_tmux_session(session: &str) -> Result<()> {
    Command::new("tmux")
        .args(["kill-session", "-t", session])
        .status()
        .with_context(|| format!("Failed to kill tmux session: {session}"))?;

    Ok(())
}

pub fn open_project_editor(cfg: &Config, project: &Project) -> Result<()> {
    run_shell_interactive(format!("{} .", cfg.editor), &project.path)
}

pub fn open_config_editor(cfg: &Config) -> Result<()> {
    let path = config::config_path()?;

    if !path.exists() {
        cfg.save()?;
    }

    let command = format!("{} {}", cfg.editor, path.display());
    run_shell_interactive(command, path.parent().unwrap_or_else(|| Path::new(".")))
}

pub fn open_lazygit(project: &Project) -> Result<()> {
    if !command_exists("lazygit") {
        anyhow::bail!("lazygit is not installed. Install it with: brew install lazygit");
    }

    run_interactive("lazygit", &[], &project.path)
}

pub fn open_local_preview(cfg: &Config, project: &Project) -> Result<()> {
    let Some(port) = project.effective_port() else {
        anyhow::bail!("No local port detected for this project.");
    };

    let Some(url) = project.localhost_url() else {
        anyhow::bail!("No local URL configured for this project.");
    };

    start_dev_server_in_project_workspace(cfg, project)?;

    if !wait_for_port(port, Duration::from_secs(20)) {
        anyhow::bail!(
            "Local server did not respond on port {port}. Check the project tmux bottom pane for errors."
        );
    }

    open_url(&url)?;

    Ok(())
}

pub fn open_deploy_preview(project: &Project) -> Result<()> {
    let Some(url) = project.deployed_url() else {
        anyhow::bail!("No deployed URL configured for this project.");
    };

    open_url(&url)?;

    Ok(())
}

fn wait_for_port(port: u16, timeout: Duration) -> bool {
    let start = Instant::now();

    while start.elapsed() < timeout {
        if TcpStream::connect(("127.0.0.1", port)).is_ok()
            || TcpStream::connect(("localhost", port)).is_ok()
        {
            return true;
        }

        thread::sleep(Duration::from_millis(250));
    }

    false
}

fn start_dev_server_in_project_workspace(cfg: &Config, project: &Project) -> Result<()> {
    if !command_exists("tmux") {
        return Ok(());
    }

    let session = project.tmux_session_name();

    if !tmux_session_exists(&session)? {
        create_tmux_workspace(cfg, project, &session)?;
    } else {
        ensure_bottom_pane(cfg, project, &session)?;
    }

    let Some(dev_command) = project.dev_server_command() else {
        anyhow::bail!("No dev script found for this project.");
    };

    let bottom_pane = pane_id(&session, 1)?;

    if pane_has_running_process(&bottom_pane)? {
        return Ok(());
    }

    let package_root = project.package_root();
    let package_root_display = package_root.display().to_string();

    let header = format!(
        "\nRunDeck preview server\n  Project: {}\n  Command: {}\n  Path: {}\n\n",
        project.name, dev_command, package_root_display
    );

    let command = format!(
        "cd {} && clear && printf {} && {}",
        shell_escape(&package_root_display),
        shell_escape(&header),
        dev_command
    );

    Command::new("tmux")
        .args(["send-keys", "-t", bottom_pane.as_str()])
        .arg(command)
        .arg("C-m")
        .status()
        .context("Failed to send dev command to project tmux pane")?;

    Ok(())
}

fn pane_has_running_process(target: &str) -> Result<bool> {
    let output = Command::new("tmux")
        .args([
            "display-message",
            "-p",
            "-t",
            target,
            "#{pane_current_command}",
        ])
        .output()
        .context("Failed to inspect tmux pane")?;

    if !output.status.success() {
        return Ok(false);
    }

    let command = String::from_utf8_lossy(&output.stdout).trim().to_string();

    Ok(!is_shell_command(&command))
}

fn is_shell_command(command: &str) -> bool {
    matches!(
        command,
        "sh" | "bash" | "zsh" | "fish" | "nu" | "pwsh" | "powershell"
    )
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn open_url(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).status()?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(url).status()?;
    }

    #[cfg(windows)]
    {
        Command::new("cmd")
            .args(["/C", "start", "", url])
            .status()?;
    }

    Ok(())
}

fn run_interactive(program: &str, args: &[&str], cwd: &Path) -> Result<()> {
    Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .with_context(|| format!("Failed to run {program}"))?;

    Ok(())
}

fn run_shell_interactive(command: String, cwd: &Path) -> Result<()> {
    #[cfg(windows)]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.args(["/C", command.as_str()]);
        c
    };

    #[cfg(not(windows))]
    let mut cmd = {
        let shell = env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
        let mut c = Command::new(shell);
        c.arg("-lc").arg(command);
        c
    };

    cmd.current_dir(cwd).status()?;

    Ok(())
}
