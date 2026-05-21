use crate::{actions, config::Config, project::Project, theme};
use anyhow::Result;
use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear as RatatuiClear, List, ListItem, ListState, Paragraph, Wrap},
};
use std::{
    io,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusPanel {
    Projects,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Overlay {
    Commands,
    Themes,
    Doctor,
    EditDeploy,
    CreateProject,
    ConfirmRemove,
}

#[derive(Debug, Clone)]
enum Action {
    AddProject,
    CreateProject,
    RemoveProject,
    Workspace,
    Editor,
    Lazygit,
    LocalPreview,
    DeployPreview,
    EditDeploy,
    Config,
    Themes,
    Doctor,
    KillSession,
    StopDev,
}

#[derive(Debug, Clone)]
struct CommandItem {
    label: String,
    hint: String,
    action: Action,
    kind: CommandKind,
}

#[derive(Debug, Clone, Copy)]
enum CommandKind {
    Primary,
    Utility,
    Danger,
}

#[derive(Debug, Clone)]
struct FlashMessage {
    text: String,
    created_at: Instant,
    ttl: Duration,
    is_error: bool,
}

struct App {
    config: Config,
    selected_project: usize,
    focus: FocusPanel,
    overlay: Option<Overlay>,
    selected_theme: usize,
    selected_command: usize,
    search_active: bool,
    search_input: String,
    info_scroll: u16,
    edit_buffer: String,
    create_buffer: String,
    flash: Option<FlashMessage>,
    started_at: Instant,
    last_prune_check: Instant,
}

impl App {
    fn new(mut config: Config) -> Self {
        let removed = config.prune_missing_projects();

        if removed > 0 {
            let _ = config.save();
        }

        config.sort_projects();

        let selected_theme = theme::all()
            .iter()
            .position(|theme| theme.id == config.theme)
            .unwrap_or(0);

        Self {
            config,
            selected_project: 0,
            focus: FocusPanel::Projects,
            overlay: None,
            selected_theme,
            selected_command: 0,
            search_active: false,
            search_input: String::new(),
            info_scroll: 0,
            edit_buffer: String::new(),
            create_buffer: String::new(),
            flash: None,
            started_at: Instant::now(),
            last_prune_check: Instant::now(),
        }
    }

    fn selected_project(&self) -> Option<&Project> {
        self.config.projects.get(self.selected_project)
    }

    fn selected_project_name(&self) -> Option<String> {
        self.selected_project().map(|project| project.name.clone())
    }

    fn reset_info_scroll(&mut self) {
        self.info_scroll = 0;
    }

    fn scroll_info_down(&mut self, amount: u16) {
        self.info_scroll = self.info_scroll.saturating_add(amount);
    }

    fn scroll_info_up(&mut self, amount: u16) {
        self.info_scroll = self.info_scroll.saturating_sub(amount);
    }

    fn cursor(&self) -> &'static str {
        if (self.started_at.elapsed().as_millis() / 500).is_multiple_of(2) {
            "█"
        } else {
            " "
        }
    }

    fn set_flash(&mut self, text: impl Into<String>, is_error: bool) {
        self.flash = Some(FlashMessage {
            text: text.into(),
            created_at: Instant::now(),
            ttl: if is_error {
                Duration::from_millis(3500)
            } else {
                Duration::from_millis(1600)
            },
            is_error,
        });
    }

    fn visible_flash(&mut self) -> Option<(String, bool)> {
        let Some(message) = &self.flash else {
            return None;
        };

        if message.created_at.elapsed() > message.ttl {
            self.flash = None;
            return None;
        }

        Some((message.text.clone(), message.is_error))
    }

    fn filtered_project_indices(&self) -> Vec<usize> {
        let query = self.search_input.trim().to_lowercase();

        if query.is_empty() {
            return (0..self.config.projects.len()).collect();
        }

        self.config
            .projects
            .iter()
            .enumerate()
            .filter_map(|(index, project)| {
                let haystack = format!(
                    "{} {} {} {}",
                    project.name,
                    project.path.display(),
                    project.stack_summary(),
                    project.package_manager()
                )
                .to_lowercase();

                haystack.contains(&query).then_some(index)
            })
            .collect()
    }

    fn selected_filtered_position(&self) -> Option<usize> {
        self.filtered_project_indices()
            .iter()
            .position(|index| *index == self.selected_project)
    }

    fn ensure_selection_visible(&mut self) {
        let filtered = self.filtered_project_indices();

        if filtered.is_empty() {
            self.selected_project = 0;
            self.reset_info_scroll();
            return;
        }

        if !filtered.contains(&self.selected_project) {
            self.selected_project = filtered[0];
            self.reset_info_scroll();
        }
    }

    fn select_project_by_name(&mut self, name: &str) {
        if let Some(index) = self
            .config
            .projects
            .iter()
            .position(|project| project.name == name)
        {
            self.selected_project = index;
            self.reset_info_scroll();
        }
    }

    fn touch_selected_project(&mut self) {
        let Some(name) = self.selected_project_name() else {
            return;
        };

        self.config.touch_project(&name);
        self.config.sort_projects();
        self.select_project_by_name(&name);

        let _ = self.config.save();
    }

    fn maybe_prune_missing_projects(&mut self) {
        if self.last_prune_check.elapsed() < Duration::from_secs(2) {
            return;
        }

        self.last_prune_check = Instant::now();

        let selected_name = self.selected_project_name();
        let removed = self.config.prune_missing_projects();

        if removed == 0 {
            return;
        }

        self.config.sort_projects();

        if let Some(name) = selected_name {
            if self
                .config
                .projects
                .iter()
                .any(|project| project.name == name)
            {
                self.select_project_by_name(&name);
            } else {
                self.selected_project = self
                    .selected_project
                    .min(self.config.projects.len().saturating_sub(1));
                self.reset_info_scroll();
            }
        }

        let _ = self.config.save();

        self.set_flash(
            format!(
                "Removed {removed} missing project{} from RunDeck",
                if removed == 1 { "" } else { "s" }
            ),
            false,
        );
    }

    fn open_deploy_editor(&mut self) {
        let Some(project) = self.selected_project() else {
            self.set_flash("No project selected", true);
            return;
        };

        self.edit_buffer = project.deploy_url.clone().unwrap_or_default();
        self.overlay = Some(Overlay::EditDeploy);
    }

    fn open_create_project_editor(&mut self) {
        self.create_buffer.clear();
        self.overlay = Some(Overlay::CreateProject);
    }

    fn open_remove_confirm(&mut self) {
        if self.selected_project().is_none() {
            self.set_flash("No project selected", true);
            return;
        }

        self.overlay = Some(Overlay::ConfirmRemove);
    }

    fn remove_selected_project(&mut self) -> Result<()> {
        let Some(name) = self.selected_project_name() else {
            self.set_flash("No project selected", true);
            return Ok(());
        };

        if self.config.remove_project(&name) {
            self.selected_project = self
                .selected_project
                .min(self.config.projects.len().saturating_sub(1));
            self.reset_info_scroll();
            self.config.save()?;
            self.set_flash(format!("Removed from RunDeck: {name}"), false);
        } else {
            self.set_flash("Project was not found in RunDeck config", true);
        }

        self.overlay = None;

        Ok(())
    }

    fn save_deploy_editor(&mut self) -> Result<()> {
        let value = self.edit_buffer.trim().to_string();

        let Some(project) = self.config.projects.get_mut(self.selected_project) else {
            self.set_flash("No project selected", true);
            return Ok(());
        };

        if value.is_empty() {
            project.deploy_url = None;
            self.set_flash("Deploy URL cleared", false);
        } else {
            project.deploy_url = Some(value);
            self.set_flash("Deploy URL saved", false);
        }

        self.config.save()?;
        self.overlay = None;
        self.edit_buffer.clear();

        Ok(())
    }

    fn command_items(&self) -> Vec<CommandItem> {
        let keymaps = &self.config.keymaps;
        let mut items = Vec::new();

        items.push(CommandItem {
            label: "Add existing project".to_string(),
            hint: keymaps.add_project.clone(),
            action: Action::AddProject,
            kind: CommandKind::Utility,
        });

        items.push(CommandItem {
            label: "Create new project".to_string(),
            hint: keymaps.create_project.clone(),
            action: Action::CreateProject,
            kind: CommandKind::Utility,
        });

        if self.selected_project().is_some() {
            items.push(CommandItem {
                label: "Remove from RunDeck".to_string(),
                hint: keymaps.remove_project.clone(),
                action: Action::RemoveProject,
                kind: CommandKind::Danger,
            });

            items.push(CommandItem {
                label: "Open tmux workspace".to_string(),
                hint: format!("{} / {}", keymaps.workspace, keymaps.workspace_alt),
                action: Action::Workspace,
                kind: CommandKind::Primary,
            });

            items.push(CommandItem {
                label: "Launch local preview".to_string(),
                hint: keymaps.local_preview.clone(),
                action: Action::LocalPreview,
                kind: CommandKind::Primary,
            });

            items.push(CommandItem {
                label: "Open deployed preview".to_string(),
                hint: keymaps.deploy_preview.clone(),
                action: Action::DeployPreview,
                kind: CommandKind::Primary,
            });

            items.push(CommandItem {
                label: "Edit deploy URL".to_string(),
                hint: keymaps.edit_deploy.clone(),
                action: Action::EditDeploy,
                kind: CommandKind::Utility,
            });

            items.push(CommandItem {
                label: "Open project in editor".to_string(),
                hint: keymaps.editor.clone(),
                action: Action::Editor,
                kind: CommandKind::Primary,
            });

            items.push(CommandItem {
                label: "Open lazygit".to_string(),
                hint: keymaps.lazygit.clone(),
                action: Action::Lazygit,
                kind: CommandKind::Primary,
            });

            items.push(CommandItem {
                label: "Stop dev server".to_string(),
                hint: keymaps.stop_dev.clone(),
                action: Action::StopDev,
                kind: CommandKind::Danger,
            });

            items.push(CommandItem {
                label: "Kill project tmux session".to_string(),
                hint: keymaps.kill_session.clone(),
                action: Action::KillSession,
                kind: CommandKind::Danger,
            });
        }

        items.push(CommandItem {
            label: "Edit RunDeck config".to_string(),
            hint: keymaps.config.clone(),
            action: Action::Config,
            kind: CommandKind::Utility,
        });

        items.push(CommandItem {
            label: "Theme picker".to_string(),
            hint: keymaps.theme.clone(),
            action: Action::Themes,
            kind: CommandKind::Utility,
        });

        items.push(CommandItem {
            label: "Run doctor".to_string(),
            hint: keymaps.doctor.clone(),
            action: Action::Doctor,
            kind: CommandKind::Utility,
        });

        items
    }
}

pub fn run(config: Config) -> Result<()> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Clear(ClearType::All))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    let mut app = App::new(config);
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if result.is_ok() {
        let mut stdout = io::stdout();
        execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
        println!("RunDeck exited.");
    }

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        app.maybe_prune_missing_projects();
        terminal.draw(|frame| draw_ui(frame, app))?;

        if !event::poll(Duration::from_millis(160))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            if matches!(key.code, KeyCode::Char('c'))
                && key.modifiers.contains(KeyModifiers::CONTROL)
            {
                break;
            }

            if let Some(overlay) = app.overlay {
                handle_overlay_key(terminal, key.code, overlay, app)?;
                continue;
            }

            if app.search_active {
                handle_search_key(key.code, app);
                continue;
            }

            let keymaps = app.config.keymaps.clone();

            if key_matches(&key.code, &keymaps.quit) || matches!(key.code, KeyCode::Esc) {
                break;
            }

            if key_matches(&key.code, &keymaps.search) {
                app.search_active = true;
                app.search_input.clear();
                continue;
            }

            if key_matches(&key.code, &keymaps.add_project) {
                run_action(terminal, app, Action::AddProject)?;
                continue;
            }

            if key_matches(&key.code, &keymaps.create_project) {
                run_action(terminal, app, Action::CreateProject)?;
                continue;
            }

            if key_matches(&key.code, &keymaps.remove_project) {
                run_action(terminal, app, Action::RemoveProject)?;
                continue;
            }

            if key_matches(&key.code, &keymaps.help) {
                app.selected_command = 0;
                app.overlay = Some(Overlay::Commands);
                continue;
            }

            if key_matches(&key.code, &keymaps.theme) {
                open_theme_picker(app);
                continue;
            }

            if key_matches(&key.code, &keymaps.doctor) {
                app.overlay = Some(Overlay::Doctor);
                continue;
            }

            if key_matches(&key.code, &keymaps.left) || matches!(key.code, KeyCode::Left) {
                app.focus = FocusPanel::Projects;
                continue;
            }

            if key_matches(&key.code, &keymaps.right) || matches!(key.code, KeyCode::Right) {
                app.focus = FocusPanel::Info;
                continue;
            }

            if matches!(key.code, KeyCode::Tab) {
                app.focus = match app.focus {
                    FocusPanel::Projects => FocusPanel::Info,
                    FocusPanel::Info => FocusPanel::Projects,
                };
                continue;
            }

            if key_matches(&key.code, &keymaps.down) || matches!(key.code, KeyCode::Down) {
                match app.focus {
                    FocusPanel::Projects => move_project_down(app),
                    FocusPanel::Info => app.scroll_info_down(1),
                }
                continue;
            }

            if key_matches(&key.code, &keymaps.up) || matches!(key.code, KeyCode::Up) {
                match app.focus {
                    FocusPanel::Projects => move_project_up(app),
                    FocusPanel::Info => app.scroll_info_up(1),
                }
                continue;
            }

            if key_matches(&key.code, &keymaps.workspace)
                || key_matches(&key.code, &keymaps.workspace_alt)
            {
                if app.selected_project().is_some() {
                    run_action(terminal, app, Action::Workspace)?;
                } else {
                    run_action(terminal, app, Action::CreateProject)?;
                }
                continue;
            }

            if key_matches(&key.code, &keymaps.local_preview) {
                run_action(terminal, app, Action::LocalPreview)?;
                continue;
            }

            if key_matches(&key.code, &keymaps.deploy_preview) {
                run_action(terminal, app, Action::DeployPreview)?;
                continue;
            }

            if key_matches(&key.code, &keymaps.edit_deploy) {
                run_action(terminal, app, Action::EditDeploy)?;
                continue;
            }

            if key_matches(&key.code, &keymaps.editor) {
                run_action(terminal, app, Action::Editor)?;
                continue;
            }

            if key_matches(&key.code, &keymaps.lazygit) {
                run_action(terminal, app, Action::Lazygit)?;
                continue;
            }

            if key_matches(&key.code, &keymaps.config) {
                run_action(terminal, app, Action::Config)?;
                continue;
            }

            if key_matches(&key.code, &keymaps.kill_session) {
                run_action(terminal, app, Action::KillSession)?;
                continue;
            }

            if key_matches(&key.code, &keymaps.stop_dev) {
                run_action(terminal, app, Action::StopDev)?;
                continue;
            }

            if key_matches(&key.code, &keymaps.reload) {
                app.config = Config::load()?;
                let removed = app.config.prune_missing_projects();
                app.config.sort_projects();
                app.ensure_selection_visible();

                if removed > 0 {
                    app.config.save()?;
                    app.set_flash(format!("Removed {removed} missing project(s)"), false);
                } else {
                    app.set_flash("Config reloaded", false);
                }
            }
        }
    }

    Ok(())
}

fn handle_search_key(code: KeyCode, app: &mut App) {
    match code {
        KeyCode::Esc => {
            app.search_active = false;
            app.search_input.clear();
            app.ensure_selection_visible();
        }
        KeyCode::Enter => {
            app.search_active = false;
            app.ensure_selection_visible();
        }
        KeyCode::Backspace => {
            app.search_input.pop();
            app.ensure_selection_visible();
        }
        KeyCode::Char(ch) => {
            app.search_input.push(ch);
            app.ensure_selection_visible();
        }
        _ => {}
    }
}

fn move_project_down(app: &mut App) {
    let filtered = app.filtered_project_indices();

    if filtered.is_empty() {
        return;
    }

    let current_position = app.selected_filtered_position().unwrap_or(0);
    let next_position = (current_position + 1).min(filtered.len() - 1);
    let next_index = filtered[next_position];

    if app.selected_project != next_index {
        app.selected_project = next_index;
        app.reset_info_scroll();
    }
}

fn move_project_up(app: &mut App) {
    let filtered = app.filtered_project_indices();

    if filtered.is_empty() {
        return;
    }

    let current_position = app.selected_filtered_position().unwrap_or(0);
    let next_position = current_position.saturating_sub(1);
    let next_index = filtered[next_position];

    if app.selected_project != next_index {
        app.selected_project = next_index;
        app.reset_info_scroll();
    }
}

fn open_theme_picker(app: &mut App) {
    app.selected_theme = theme::all()
        .iter()
        .position(|theme| theme.id == app.config.theme)
        .unwrap_or(0);

    app.overlay = Some(Overlay::Themes);
}

fn handle_overlay_key(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    code: KeyCode,
    overlay: Overlay,
    app: &mut App,
) -> Result<()> {
    let keymaps = app.config.keymaps.clone();

    match overlay {
        Overlay::Commands => {
            if key_matches(&code, &keymaps.quit)
                || key_matches(&code, &keymaps.help)
                || matches!(code, KeyCode::Esc)
            {
                app.overlay = None;
                return Ok(());
            }

            if key_matches(&code, &keymaps.down) || matches!(code, KeyCode::Down) {
                let commands = app.command_items();

                if !commands.is_empty() {
                    app.selected_command = (app.selected_command + 1).min(commands.len() - 1);
                }

                return Ok(());
            }

            if key_matches(&code, &keymaps.up) || matches!(code, KeyCode::Up) {
                app.selected_command = app.selected_command.saturating_sub(1);
                return Ok(());
            }

            if matches!(code, KeyCode::Enter) {
                let action = app
                    .command_items()
                    .get(app.selected_command)
                    .map(|command| command.action.clone());

                if let Some(action) = action {
                    app.overlay = None;
                    run_action(terminal, app, action)?;
                }

                return Ok(());
            }

            if key_matches(&code, &keymaps.add_project) {
                app.overlay = None;
                run_action(terminal, app, Action::AddProject)?;
            } else if key_matches(&code, &keymaps.create_project) {
                app.overlay = None;
                run_action(terminal, app, Action::CreateProject)?;
            } else if key_matches(&code, &keymaps.remove_project) {
                app.overlay = None;
                run_action(terminal, app, Action::RemoveProject)?;
            } else if key_matches(&code, &keymaps.workspace)
                || key_matches(&code, &keymaps.workspace_alt)
            {
                app.overlay = None;
                run_action(terminal, app, Action::Workspace)?;
            } else if key_matches(&code, &keymaps.local_preview) {
                app.overlay = None;
                run_action(terminal, app, Action::LocalPreview)?;
            } else if key_matches(&code, &keymaps.deploy_preview) {
                app.overlay = None;
                run_action(terminal, app, Action::DeployPreview)?;
            } else if key_matches(&code, &keymaps.edit_deploy) {
                app.overlay = None;
                run_action(terminal, app, Action::EditDeploy)?;
            } else if key_matches(&code, &keymaps.editor) {
                app.overlay = None;
                run_action(terminal, app, Action::Editor)?;
            } else if key_matches(&code, &keymaps.lazygit) {
                app.overlay = None;
                run_action(terminal, app, Action::Lazygit)?;
            } else if key_matches(&code, &keymaps.config) {
                app.overlay = None;
                run_action(terminal, app, Action::Config)?;
            } else if key_matches(&code, &keymaps.kill_session) {
                app.overlay = None;
                run_action(terminal, app, Action::KillSession)?;
            } else if key_matches(&code, &keymaps.stop_dev) {
                app.overlay = None;
                run_action(terminal, app, Action::StopDev)?;
            } else if key_matches(&code, &keymaps.doctor) {
                app.overlay = Some(Overlay::Doctor);
            } else if key_matches(&code, &keymaps.theme) {
                open_theme_picker(app);
            }
        }

        Overlay::Themes => {
            if key_matches(&code, &keymaps.quit) || matches!(code, KeyCode::Esc) {
                app.overlay = None;
                return Ok(());
            }

            if key_matches(&code, &keymaps.down) || matches!(code, KeyCode::Down) {
                let themes = theme::all();
                app.selected_theme = (app.selected_theme + 1).min(themes.len() - 1);
                return Ok(());
            }

            if key_matches(&code, &keymaps.up) || matches!(code, KeyCode::Up) {
                app.selected_theme = app.selected_theme.saturating_sub(1);
                return Ok(());
            }

            if matches!(code, KeyCode::Enter) {
                let themes = theme::all();

                if let Some(selected) = themes.get(app.selected_theme) {
                    app.config.theme = selected.id.to_string();
                    app.config.save()?;
                    app.set_flash(format!("Theme: {}", selected.label), false);
                }

                app.overlay = None;
            }
        }

        Overlay::Doctor => {
            if key_matches(&code, &keymaps.quit)
                || key_matches(&code, &keymaps.doctor)
                || matches!(code, KeyCode::Esc)
            {
                app.overlay = None;
            }
        }

        Overlay::EditDeploy => match code {
            KeyCode::Esc => {
                app.overlay = None;
                app.edit_buffer.clear();
            }
            KeyCode::Enter => {
                app.save_deploy_editor()?;
            }
            KeyCode::Backspace => {
                app.edit_buffer.pop();
            }
            KeyCode::Char(ch) => {
                app.edit_buffer.push(ch);
            }
            _ => {}
        },

        Overlay::CreateProject => match code {
            KeyCode::Esc => {
                app.overlay = None;
                app.create_buffer.clear();
            }
            KeyCode::Enter => {
                let name = app.create_buffer.trim().to_string();
                app.overlay = None;
                app.create_buffer.clear();

                let result = suspend_terminal(terminal, || {
                    actions::create_project_with_picker(&mut app.config, &name)
                });

                match result {
                    Ok(Some(project_name)) => {
                        app.config.sort_projects();
                        app.select_project_by_name(&project_name);
                        app.set_flash(format!("Created project: {project_name}"), false);
                    }
                    Ok(None) => {
                        app.set_flash("Create project cancelled", false);
                    }
                    Err(error) => {
                        app.set_flash(format!("Error: {error}"), true);
                    }
                }
            }
            KeyCode::Backspace => {
                app.create_buffer.pop();
            }
            KeyCode::Char(ch) => {
                app.create_buffer.push(ch);
            }
            _ => {}
        },

        Overlay::ConfirmRemove => match code {
            KeyCode::Esc => {
                app.overlay = None;
            }
            KeyCode::Enter => {
                app.remove_selected_project()?;
            }
            _ => {}
        },
    }

    Ok(())
}

fn run_action(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    action: Action,
) -> Result<()> {
    if matches!(action, Action::AddProject) {
        let result = suspend_terminal(terminal, || {
            actions::add_project_with_picker(&mut app.config)
        });

        match result {
            Ok(Some(name)) => {
                app.config.sort_projects();
                app.select_project_by_name(&name);
                app.set_flash(format!("Added project: {name}"), false);
            }
            Ok(None) => {
                app.set_flash("Add project cancelled", false);
            }
            Err(error) => {
                app.set_flash(format!("Error: {error}"), true);
            }
        }

        return Ok(());
    }

    if matches!(action, Action::CreateProject) {
        app.open_create_project_editor();
        return Ok(());
    }

    if matches!(action, Action::RemoveProject) {
        app.open_remove_confirm();
        return Ok(());
    }

    if matches!(action, Action::EditDeploy) {
        app.open_deploy_editor();
        return Ok(());
    }

    let should_touch_project = matches!(
        action,
        Action::Workspace
            | Action::Editor
            | Action::Lazygit
            | Action::LocalPreview
            | Action::DeployPreview
    );

    let result = match action {
        Action::Config => {
            let result = suspend_terminal(terminal, || actions::open_config_editor(&app.config));
            app.config = Config::load().unwrap_or_else(|_| app.config.clone());
            let removed = app.config.prune_missing_projects();
            app.config.sort_projects();

            if removed > 0 {
                let _ = app.config.save();
            }

            result
        }

        Action::Themes => {
            open_theme_picker(app);
            Ok(())
        }

        Action::Doctor => {
            app.overlay = Some(Overlay::Doctor);
            Ok(())
        }

        Action::Workspace
        | Action::Editor
        | Action::Lazygit
        | Action::LocalPreview
        | Action::DeployPreview
        | Action::KillSession
        | Action::StopDev => {
            let Some(project) = app.selected_project().cloned() else {
                app.set_flash("No project selected", true);
                return Ok(());
            };

            match action {
                Action::Workspace => {
                    suspend_terminal(terminal, || actions::open_workspace(&app.config, &project))
                }
                Action::Editor => suspend_terminal(terminal, || {
                    actions::open_project_editor(&app.config, &project)
                }),
                Action::Lazygit => suspend_terminal(terminal, || actions::open_lazygit(&project)),
                Action::LocalPreview => actions::open_local_preview(&app.config, &project),
                Action::DeployPreview => actions::open_deploy_preview(&project),
                Action::KillSession => actions::kill_project_session(&project),
                Action::StopDev => actions::stop_project_dev(&project),
                Action::AddProject
                | Action::CreateProject
                | Action::RemoveProject
                | Action::EditDeploy
                | Action::Config
                | Action::Themes
                | Action::Doctor => Ok(()),
            }
        }

        Action::AddProject | Action::CreateProject | Action::RemoveProject | Action::EditDeploy => {
            Ok(())
        }
    };

    match result {
        Ok(_) => {
            if should_touch_project {
                app.touch_selected_project();
            }

            app.set_flash("Done", false);
        }
        Err(error) => {
            app.set_flash(format!("Error: {error}"), true);
        }
    }

    Ok(())
}

fn suspend_terminal<F, T>(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, f: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        Clear(ClearType::All),
        MoveTo(0, 0)
    )?;
    terminal.show_cursor()?;

    let result = f();

    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        Clear(ClearType::All),
        MoveTo(0, 0)
    )?;
    enable_raw_mode()?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    result
}

fn draw_ui(frame: &mut Frame<'_>, app: &mut App) {
    let current_theme = theme::get(&app.config.theme);
    let area = frame.area();

    frame.render_widget(
        Block::default().style(Style::default().bg(current_theme.bg).fg(current_theme.text)),
        area,
    );

    let shell = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(current_theme.border))
        .style(Style::default().bg(current_theme.bg).fg(current_theme.text));

    let inner = shell.inner(area);
    frame.render_widget(shell, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .margin(1)
        .split(inner);

    draw_header(frame, app, layout[0], &current_theme);

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Length(2),
            Constraint::Percentage(66),
        ])
        .split(layout[1]);

    draw_projects(frame, app, main[0], &current_theme);
    draw_project_info(frame, app, main[2], &current_theme);
    draw_dashboard_footer(frame, app, layout[2], &current_theme);

    match app.overlay {
        Some(Overlay::Commands) => draw_commands(frame, app, area, &current_theme),
        Some(Overlay::Themes) => draw_themes(frame, app, area, &current_theme),
        Some(Overlay::Doctor) => draw_doctor(frame, app, area, &current_theme),
        Some(Overlay::EditDeploy) => draw_deploy_editor(frame, app, area, &current_theme),
        Some(Overlay::CreateProject) => {
            draw_create_project_editor(frame, app, area, &current_theme)
        }
        Some(Overlay::ConfirmRemove) => draw_remove_confirm(frame, app, area, &current_theme),
        None => {}
    }
}

fn draw_header(frame: &mut Frame<'_>, app: &mut App, area: Rect, current_theme: &theme::Theme) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let logo_lines = vec![
        Line::from(vec![Span::styled(
            "██████╗ ██╗   ██╗███╗   ██╗██████╗ ███████╗ ██████╗██╗  ██╗",
            Style::default()
                .fg(current_theme.accent)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "██╔══██╗██║   ██║████╗  ██║██╔══██╗██╔════╝██╔════╝██║ ██╔╝",
            Style::default()
                .fg(current_theme.accent)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "██████╔╝██║   ██║██╔██╗ ██║██║  ██║█████╗  ██║     █████╔╝ ",
            Style::default()
                .fg(current_theme.accent)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "██╔══██╗██║   ██║██║╚██╗██║██║  ██║██╔══╝  ██║     ██╔═██╗ ",
            Style::default()
                .fg(current_theme.accent)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "██║  ██║╚██████╔╝██║ ╚████║██████╔╝███████╗╚██████╗██║  ██╗",
            Style::default()
                .fg(current_theme.accent)
                .add_modifier(Modifier::BOLD),
        )]),
    ];

    let logo = Paragraph::new(logo_lines)
        .alignment(Alignment::Center)
        .style(Style::default().bg(current_theme.bg));

    let subtitle = Paragraph::new(Line::from(vec![Span::styled(
        "Personal Project Dashboard",
        Style::default()
            .fg(current_theme.muted)
            .add_modifier(Modifier::ITALIC),
    )]))
    .alignment(Alignment::Center)
    .style(
        Style::default()
            .bg(current_theme.bg)
            .fg(current_theme.muted),
    );

    let message_line = if app.search_active {
        Line::from(vec![
            Span::styled("/", Style::default().fg(current_theme.accent)),
            Span::styled(
                app.search_input.clone(),
                Style::default().fg(current_theme.text),
            ),
        ])
    } else if let Some((message, is_error)) = app.visible_flash() {
        let style = if is_error {
            Style::default().fg(current_theme.danger)
        } else {
            Style::default().fg(current_theme.success)
        };

        Line::from(vec![Span::styled(message, style)])
    } else {
        Line::from("")
    };

    let message = Paragraph::new(message_line)
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .bg(current_theme.bg)
                .fg(current_theme.muted),
        );

    let divider = Paragraph::new(Line::from(vec![Span::styled(
        "─".repeat(area.width as usize),
        Style::default().fg(current_theme.border),
    )]))
    .style(Style::default().bg(current_theme.bg));

    frame.render_widget(logo, rows[0]);
    frame.render_widget(subtitle, rows[1]);
    frame.render_widget(message, rows[2]);
    frame.render_widget(divider, rows[3]);
}

fn draw_projects(frame: &mut Frame<'_>, app: &mut App, area: Rect, current_theme: &theme::Theme) {
    let filtered = app.filtered_project_indices();

    let items: Vec<ListItem> = if app.config.projects.is_empty() {
        vec![
            ListItem::new(Line::from(vec![Span::styled(
                "No projects yet",
                Style::default()
                    .fg(current_theme.warning)
                    .add_modifier(Modifier::BOLD),
            )])),
            ListItem::new(Line::from("")),
            ListItem::new(Line::from(format!(
                "{} Create new project",
                app.config.keymaps.create_project
            ))),
            ListItem::new(Line::from(format!(
                "{} Add existing project",
                app.config.keymaps.add_project
            ))),
        ]
    } else if filtered.is_empty() {
        vec![
            ListItem::new(Line::from(vec![Span::styled(
                "No matches",
                Style::default()
                    .fg(current_theme.warning)
                    .add_modifier(Modifier::BOLD),
            )])),
            ListItem::new(Line::from("")),
            ListItem::new(Line::from("Press Esc to clear search")),
        ]
    } else {
        filtered
            .iter()
            .filter_map(|index| app.config.projects.get(*index))
            .map(|project| {
                let icon = if app.config.show_icons { "󰏗  " } else { "" };

                ListItem::new(Line::from(vec![
                    Span::raw(icon),
                    Span::raw(project.name.clone()),
                ]))
            })
            .collect()
    };

    let mut state = ListState::default();

    if !filtered.is_empty() {
        state.select(app.selected_filtered_position());
    }

    let border_style = if app.focus == FocusPanel::Projects {
        Style::default().fg(current_theme.accent)
    } else {
        Style::default().fg(current_theme.border)
    };

    let title = if app.search_active && !app.search_input.is_empty() {
        format!(" Projects — /{} ", app.search_input)
    } else {
        " Projects ".to_string()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style)
                .style(
                    Style::default()
                        .bg(current_theme.surface)
                        .fg(current_theme.text),
                ),
        )
        .highlight_symbol("▶ ")
        .highlight_style(
            Style::default()
                .bg(current_theme.highlight_bg)
                .fg(current_theme.accent)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_project_info(
    frame: &mut Frame<'_>,
    app: &mut App,
    area: Rect,
    current_theme: &theme::Theme,
) {
    let border_style = if app.focus == FocusPanel::Info {
        Style::default().fg(current_theme.accent)
    } else {
        Style::default().fg(current_theme.border)
    };

    let scroll_hint = if app.focus == FocusPanel::Info {
        " Project Info · j/k scroll "
    } else {
        " Project Info "
    };

    let block = Block::default()
        .title(scroll_hint)
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(
            Style::default()
                .bg(current_theme.surface)
                .fg(current_theme.text),
        );

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = if let Some(project) = app.selected_project() {
        let keymaps = &app.config.keymaps;
        let dev_status = actions::dev_server_status(project);
        let dev_style = if dev_status == "Running" {
            Style::default().fg(current_theme.success)
        } else {
            Style::default().fg(current_theme.warning)
        };

        let mut lines = vec![
            Line::from(vec![Span::styled(
                project.name.clone(),
                Style::default()
                    .fg(current_theme.accent)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(project.path.display().to_string()),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Stack",
                Style::default()
                    .fg(current_theme.muted)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(project.stack_summary()),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Environment",
                Style::default()
                    .fg(current_theme.muted)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("Package  ", Style::default().fg(current_theme.muted)),
                Span::raw(project.package_manager()),
            ]),
            Line::from(vec![
                Span::styled("Dev      ", Style::default().fg(current_theme.muted)),
                Span::styled(dev_status, dev_style),
            ]),
        ];

        if let Some(port) = project.effective_port() {
            lines.push(Line::from(vec![
                Span::styled("Port     ", Style::default().fg(current_theme.muted)),
                Span::raw(port.to_string()),
            ]));
        }

        let urls = project.urls();
        let has_deploy = urls.iter().any(|(label, _)| label == "Deploy");

        for (label, url) in urls {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{label:<9}"),
                    Style::default().fg(current_theme.muted),
                ),
                Span::raw(url),
            ]));
        }

        if !has_deploy {
            lines.push(Line::from(vec![
                Span::styled("Deploy   ", Style::default().fg(current_theme.muted)),
                Span::styled(
                    format!("<empty> press {} to set", keymaps.edit_deploy),
                    Style::default().fg(current_theme.warning),
                ),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Quick Actions",
            Style::default()
                .fg(current_theme.muted)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(vec![
            Span::styled(
                keymaps.workspace.clone(),
                Style::default().fg(current_theme.success),
            ),
            Span::raw(" workspace   "),
            Span::styled(
                keymaps.local_preview.clone(),
                Style::default().fg(current_theme.accent),
            ),
            Span::raw(" local   "),
            Span::styled(
                keymaps.deploy_preview.clone(),
                Style::default().fg(current_theme.accent),
            ),
            Span::raw(" deploy   "),
            Span::styled(
                keymaps.edit_deploy.clone(),
                Style::default().fg(current_theme.accent),
            ),
            Span::raw(" edit deploy"),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                keymaps.add_project.clone(),
                Style::default().fg(current_theme.accent),
            ),
            Span::raw(" add · "),
            Span::styled(
                keymaps.create_project.clone(),
                Style::default().fg(current_theme.accent),
            ),
            Span::raw(" create · "),
            Span::styled(
                keymaps.remove_project.clone(),
                Style::default().fg(current_theme.danger),
            ),
            Span::raw(" remove · "),
            Span::styled(
                keymaps.lazygit.clone(),
                Style::default().fg(current_theme.accent),
            ),
            Span::raw(" git"),
        ]));

        lines
    } else {
        vec![
            Line::from(vec![Span::styled(
                "Welcome to RunDeck",
                Style::default()
                    .fg(current_theme.accent)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("A fast terminal launcher for dev projects."),
            Line::from(""),
            Line::from(format!(
                "{} Create a new project folder",
                app.config.keymaps.create_project
            )),
            Line::from(format!(
                "{} Add an existing project",
                app.config.keymaps.add_project
            )),
            Line::from(""),
            Line::from("Create opens a folder picker, creates the folder,"),
            Line::from("adds it to RunDeck, then opens tmux + nvim."),
        ]
    };

    let max_scroll = lines.len().saturating_sub(inner.height as usize) as u16;
    app.info_scroll = app.info_scroll.min(max_scroll);

    let body = Paragraph::new(lines)
        .style(
            Style::default()
                .bg(current_theme.surface)
                .fg(current_theme.text),
        )
        .scroll((app.info_scroll, 0))
        .wrap(Wrap { trim: true });

    frame.render_widget(body, inner);
}

fn draw_dashboard_footer(
    frame: &mut Frame<'_>,
    app: &App,
    area: Rect,
    current_theme: &theme::Theme,
) {
    let keymaps = &app.config.keymaps;

    let padded_area = Rect {
        x: area.x,
        y: area.y + 1,
        width: area.width,
        height: area.height.saturating_sub(1),
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(68), Constraint::Percentage(32)])
        .split(padded_area);

    let left = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("v{}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(current_theme.success),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} Search", keymaps.search),
            Style::default().fg(current_theme.accent),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} Add", keymaps.add_project),
            Style::default().fg(current_theme.accent),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} Create", keymaps.create_project),
            Style::default().fg(current_theme.accent),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} Remove", keymaps.remove_project),
            Style::default().fg(current_theme.danger),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} Help", keymaps.help),
            Style::default().fg(current_theme.accent),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} Quit", keymaps.quit),
            Style::default().fg(current_theme.danger),
        ),
    ]))
    .alignment(Alignment::Left)
    .style(
        Style::default()
            .bg(current_theme.bg)
            .fg(current_theme.muted),
    );

    let right = Paragraph::new(Line::from(vec![
        Span::styled("Theme: ", Style::default().fg(current_theme.muted)),
        Span::styled(
            app.config.theme.clone(),
            Style::default().fg(current_theme.accent),
        ),
    ]))
    .alignment(Alignment::Right)
    .style(
        Style::default()
            .bg(current_theme.bg)
            .fg(current_theme.muted),
    );

    frame.render_widget(left, chunks[0]);
    frame.render_widget(right, chunks[1]);
}

fn draw_commands(frame: &mut Frame<'_>, app: &mut App, area: Rect, current_theme: &theme::Theme) {
    let popup = centered_rect(66, 62, area);
    frame.render_widget(RatatuiClear, popup);

    let commands = app.command_items();

    let items: Vec<ListItem> = commands
        .iter()
        .map(|command| {
            let style = match command.kind {
                CommandKind::Primary => Style::default().fg(current_theme.text),
                CommandKind::Utility => Style::default().fg(current_theme.accent),
                CommandKind::Danger => Style::default().fg(current_theme.danger),
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<42}", command.label), style),
                Span::styled(
                    command.hint.clone(),
                    Style::default().fg(current_theme.muted),
                ),
            ]))
        })
        .collect();

    let mut state = ListState::default();

    if !commands.is_empty() {
        state.select(Some(app.selected_command.min(commands.len() - 1)));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Commands — Enter run · j/k move · q close ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(current_theme.accent))
                .style(
                    Style::default()
                        .bg(current_theme.surface)
                        .fg(current_theme.text),
                ),
        )
        .highlight_symbol("▶ ")
        .highlight_style(
            Style::default()
                .bg(current_theme.highlight_bg)
                .fg(current_theme.success)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, popup, &mut state);
}

fn draw_themes(frame: &mut Frame<'_>, app: &mut App, area: Rect, current_theme: &theme::Theme) {
    let popup = centered_rect(44, 58, area);
    frame.render_widget(RatatuiClear, popup);

    let available_themes = theme::all();

    let items: Vec<ListItem> = available_themes
        .iter()
        .map(|theme_item| {
            let marker = if theme_item.id == app.config.theme {
                "● "
            } else {
                "  "
            };

            ListItem::new(Line::from(format!("{marker}{}", theme_item.label)))
        })
        .collect();

    let mut state = ListState::default();

    if !items.is_empty() {
        state.select(Some(app.selected_theme.min(items.len() - 1)));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Themes — Enter apply · q close ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(current_theme.accent))
                .style(
                    Style::default()
                        .bg(current_theme.surface)
                        .fg(current_theme.text),
                ),
        )
        .highlight_symbol("▶ ")
        .highlight_style(
            Style::default()
                .bg(current_theme.highlight_bg)
                .fg(current_theme.accent)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, popup, &mut state);
}

fn draw_doctor(frame: &mut Frame<'_>, app: &mut App, area: Rect, current_theme: &theme::Theme) {
    let popup = centered_rect(60, 56, area);
    frame.render_widget(RatatuiClear, popup);

    let mut lines = vec![
        Line::from(vec![Span::styled(
            "RunDeck Doctor",
            Style::default()
                .fg(current_theme.accent)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];

    for item in actions::doctor_items(&app.config) {
        let marker = if item.ok { "✓" } else { "✗" };
        let marker_style = if item.ok {
            Style::default().fg(current_theme.success)
        } else {
            Style::default().fg(current_theme.danger)
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{marker} "), marker_style),
            Span::styled(
                format!("{:<24}", item.name),
                Style::default().fg(current_theme.text),
            ),
            Span::styled(item.note, Style::default().fg(current_theme.muted)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            app.config.keymaps.quit.clone(),
            Style::default().fg(current_theme.accent),
        ),
        Span::raw("/Esc close"),
    ]));

    let doctor = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Doctor ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(current_theme.accent))
                .style(
                    Style::default()
                        .bg(current_theme.surface)
                        .fg(current_theme.text),
                ),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(doctor, popup);
}

fn draw_deploy_editor(
    frame: &mut Frame<'_>,
    app: &mut App,
    area: Rect,
    current_theme: &theme::Theme,
) {
    let popup = centered_rect(66, 22, area);
    frame.render_widget(RatatuiClear, popup);

    let project_name = app
        .selected_project()
        .map(|project| project.name.clone())
        .unwrap_or_else(|| "Project".to_string());

    let lines = vec![
        Line::from(vec![Span::styled(
            project_name,
            Style::default()
                .fg(current_theme.accent)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Deploy URL  ", Style::default().fg(current_theme.muted)),
            Span::raw(app.edit_buffer.clone()),
            Span::styled(app.cursor(), Style::default().fg(current_theme.accent)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(current_theme.success)),
            Span::raw(" save · "),
            Span::styled("Esc", Style::default().fg(current_theme.danger)),
            Span::raw(" cancel · "),
            Span::styled("Backspace", Style::default().fg(current_theme.accent)),
            Span::raw(" delete"),
        ]),
    ];

    let editor = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Edit Deploy URL ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(current_theme.accent))
                .style(
                    Style::default()
                        .bg(current_theme.surface)
                        .fg(current_theme.text),
                ),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(editor, popup);
}

fn draw_create_project_editor(
    frame: &mut Frame<'_>,
    app: &mut App,
    area: Rect,
    current_theme: &theme::Theme,
) {
    let popup = centered_rect(62, 24, area);
    frame.render_widget(RatatuiClear, popup);

    let lines = vec![
        Line::from(vec![Span::styled(
            "New project folder",
            Style::default()
                .fg(current_theme.accent)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Name  ", Style::default().fg(current_theme.muted)),
            Span::raw(app.create_buffer.clone()),
            Span::styled(app.cursor(), Style::default().fg(current_theme.accent)),
        ]),
        Line::from(""),
        Line::from("After pressing Enter, choose where to create it."),
        Line::from("RunDeck will add it and open tmux + nvim."),
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(current_theme.success)),
            Span::raw(" continue · "),
            Span::styled("Esc", Style::default().fg(current_theme.danger)),
            Span::raw(" cancel · "),
            Span::styled("Backspace", Style::default().fg(current_theme.accent)),
            Span::raw(" delete"),
        ]),
    ];

    let editor = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Create Project ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(current_theme.accent))
                .style(
                    Style::default()
                        .bg(current_theme.surface)
                        .fg(current_theme.text),
                ),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(editor, popup);
}

fn draw_remove_confirm(
    frame: &mut Frame<'_>,
    app: &mut App,
    area: Rect,
    current_theme: &theme::Theme,
) {
    let popup = centered_rect(58, 22, area);
    frame.render_widget(RatatuiClear, popup);

    let project_name = app
        .selected_project()
        .map(|project| project.name.clone())
        .unwrap_or_else(|| "Project".to_string());

    let lines = vec![
        Line::from(vec![Span::styled(
            "Remove from RunDeck?",
            Style::default()
                .fg(current_theme.danger)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Project  ", Style::default().fg(current_theme.muted)),
            Span::styled(project_name, Style::default().fg(current_theme.accent)),
        ]),
        Line::from(""),
        Line::from("This only removes it from the dashboard/config."),
        Line::from("The project folder on disk will NOT be deleted."),
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(current_theme.success)),
            Span::raw(" remove · "),
            Span::styled("Esc", Style::default().fg(current_theme.danger)),
            Span::raw(" cancel"),
        ]),
    ];

    let popup_widget = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Confirm Remove ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(current_theme.danger))
                .style(
                    Style::default()
                        .bg(current_theme.surface)
                        .fg(current_theme.text),
                ),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(popup_widget, popup);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn key_matches(code: &KeyCode, binding: &str) -> bool {
    let binding = binding.trim();

    if binding.is_empty() {
        return false;
    }

    if binding.chars().count() == 1 {
        return binding
            .chars()
            .next()
            .map(|ch| matches!(code, KeyCode::Char(actual) if *actual == ch))
            .unwrap_or(false);
    }

    match binding.to_lowercase().as_str() {
        "enter" | "return" => matches!(code, KeyCode::Enter),
        "esc" | "escape" => matches!(code, KeyCode::Esc),
        "tab" => matches!(code, KeyCode::Tab),
        "space" => matches!(code, KeyCode::Char(' ')),
        "backspace" => matches!(code, KeyCode::Backspace),
        "up" => matches!(code, KeyCode::Up),
        "down" => matches!(code, KeyCode::Down),
        "left" => matches!(code, KeyCode::Left),
        "right" => matches!(code, KeyCode::Right),
        _ => false,
    }
}
