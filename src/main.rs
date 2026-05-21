mod actions;
mod config;
mod project;
mod theme;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rundeck")]
#[command(version, about = "A terminal dashboard for launching dev projects")]
struct Cli {
    #[arg(long, hide = true)]
    dashboard: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Add {
        path: PathBuf,

        #[arg(short, long)]
        name: Option<String>,

        #[arg(short, long)]
        port: Option<u16>,

        #[arg(long)]
        url: Option<String>,
    },
    List,
    Remove {
        name: String,
    },
    Open {
        name: String,
    },
    Back,
    Close,
    Kill {
        name: Option<String>,
    },
    Doctor,
    Config,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Add {
            path,
            name,
            port,
            url,
        }) => {
            let mut cfg = config::Config::load()?;
            cfg.add_project(path, name, port, url)?;
            cfg.sort_projects();
            cfg.save()?;
            println!("Project added.");
        }

        Some(Commands::List) => {
            let mut cfg = config::Config::load()?;
            cfg.sort_projects();

            if cfg.projects.is_empty() {
                println!("No projects added yet.");
                println!("Run: rundeck add ~/Projects/my-app --name \"My App\" --port 3000");
                return Ok(());
            }

            for project in cfg.projects {
                let port = project.port.map(|p| format!(" :{}", p)).unwrap_or_default();
                let deploy = project
                    .deploy_url
                    .as_ref()
                    .map(|url| format!(" — {}", url))
                    .unwrap_or_default();

                println!(
                    "{}{} — {}{}",
                    project.name,
                    port,
                    project.path.display(),
                    deploy
                );
            }
        }

        Some(Commands::Remove { name }) => {
            let mut cfg = config::Config::load()?;
            cfg.remove_project(&name);
            cfg.save()?;
            println!("Project removed.");
        }

        Some(Commands::Open { name }) => {
            let mut cfg = config::Config::load()?;
            let project = cfg
                .project_by_name(&name)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Project not found: {}", name))?;

            cfg.touch_project(&project.name);
            cfg.sort_projects();
            cfg.save()?;

            actions::open_workspace(&cfg, &project)?;
        }

        Some(Commands::Back) => {
            let cfg = config::Config::load()?;
            actions::back_to_rundeck(&cfg)?;
        }

        Some(Commands::Close) => {
            let cfg = config::Config::load()?;
            actions::close_current_session(&cfg)?;
        }

        Some(Commands::Kill { name }) => {
            let cfg = config::Config::load()?;
            actions::kill_session_command(&cfg, name)?;
        }

        Some(Commands::Doctor) => {
            actions::doctor(&config::Config::load()?);
        }

        Some(Commands::Config) => {
            let cfg = config::Config::load()?;
            actions::open_config_editor(&cfg)?;
        }

        None => {
            if !cli.dashboard && !actions::is_inside_tmux() {
                actions::open_dashboard_session()?;
                return Ok(());
            }

            let mut cfg = config::Config::load()?;
            actions::remember_current_rundeck_session(&mut cfg);
            cfg.sort_projects();
            cfg.save()?;

            ui::run(cfg)?;
        }
    }

    Ok(())
}
