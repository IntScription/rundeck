# RunDeck

RunDeck is a fast terminal dashboard for managing personal development projects.

It helps you open project workspaces, tmux sessions, Neovim, lazygit, local previews, deploy links, and project metadata from one clean terminal UI.

![RunDeck Preview](./assets/rundeck-preview.png)

## Why RunDeck?

Most developers keep switching between:

- terminal folders
- tmux sessions
- Neovim
- lazygit
- localhost URLs
- deploy links
- project notes/configs

RunDeck brings all of that into one keyboard-driven dashboard.

## Features

- Terminal dashboard built in Rust
- Project launcher with tmux + Neovim workspace
- Automatic stack detection
- Local preview launcher
- Deploy URL launcher
- lazygit shortcut
- Add existing projects with fzf
- Create new projects from the dashboard
- Remove projects from RunDeck without deleting folders
- Auto-removes missing projects when folders are deleted
- Configurable keymaps
- Multiple themes
- Optional Neovim/LazyVim companion plugin

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/IntScription/rundeck/main/scripts/install.sh | bash
```

### Requirements

RunDeck works best with:

- Rust / Cargo
- tmux
- git
- fzf
- lazygit
- Neovim

On macOS:

```bash
brew install tmux lazygit fzf
```

### Or tap first

```bash
brew tap IntScription/rundeck
brew install rundeck
```

### Universal install script

```bash
curl -fsSL https://raw.githubusercontent.com/IntScription/rundeck/main/scripts/install.sh | bash
```

If `rundeck` is not found after installing, add Cargo/local binaries to your shell path:

```bash
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
```

For zsh:

```bash
echo 'export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### From source

```bash
cargo install --git https://github.com/IntScription/rundeck --force
```

Or from a local clone:

```bash
git clone https://github.com/IntScription/rundeck.git
cd rundeck
cargo install --path . --force
```

## Requirements

RunDeck works best with:

- tmux
- git
- fzf
- lazygit
- Neovim

On macOS:

```bash
brew install tmux lazygit fzf
```

Check your setup:

```bash
rundeck doctor
```

## Usage

Open the dashboard:

```bash
rundeck
```

Run diagnostics:

```bash
rundeck doctor
```

Add a project manually:

```bash
rundeck add ~/Projects/my-app --name "My App" --port 3000
```

Add a deploy URL:

```bash
rundeck add ~/Projects/my-app \
  --name "My App" \
  --port 3000 \
  --url "https://my-app.vercel.app"
```

## Dashboard Keymaps

Default keymaps:

| Key | Action |
|---|---|
| `Enter` | Open project tmux workspace |
| `a` | Add existing project |
| `c` | Create new project |
| `d` | Remove project from RunDeck only |
| `b` | Start/open local preview |
| `B` | Open deployed preview |
| `g` | Open lazygit |
| `u` | Edit deploy URL |
| `e` | Edit RunDeck config |
| `T` | Theme picker |
| `D` | Doctor |
| `/` | Search projects |
| `?` | Help / commands |
| `q` | Quit |
| `h/l` | Switch focus |
| `j/k` | Move or scroll |

Removing a project with `d` only removes it from RunDeck config. It does not delete the actual project folder.

## tmux Workspace

Pressing `Enter` opens a tmux workspace for the selected project.

By default:

- Top pane opens your editor
- Bottom pane opens a shell
- The bottom pane shows useful RunDeck commands

Inside a project tmux session:

```bash
rundeck back
```

Return to RunDeck.

```bash
rundeck close
```

Return to RunDeck and close the current project session.

```bash
rundeck kill
```

Kill the current tmux session.

## Local Preview

Press `b` to start the project dev server and open localhost.

RunDeck detects common dev servers:

- Next.js → `3000`
- Vite → `5173`
- Expo → `8081`

For custom setups, edit config:

```toml
[[projects]]
name = "My App"
path = "/Users/me/Projects/my-app"
port = 3000
dev_command = "cd web && npm run dev"
```

## Monorepo / Workspace Support

RunDeck supports projects like:

```txt
my-project/
├─ web/
│  └─ package.json
├─ mobile/
│  └─ package.json
└─ supabase/
```

It shows one project in the dashboard:

```txt
My Project
```

The stack detector reads from root, `web`, `mobile`, `apps`, `packages`, Supabase, Tauri, Rust, Python, Go, and other common project markers.

Example detected stack:

```txt
Next.js · React · TypeScript · Tailwind · Expo · React Native · Supabase
```

## Config

Config lives here:

```txt
~/.config/rundeck/config.toml
```

Open it from RunDeck with:

```txt
e
```

Example config:

```toml
editor = "nvim"
shell = "/bin/zsh"
theme = "catppuccin-mocha"
top_pane_ratio = 70
show_icons = true
project_picker = "fzf"
project_roots = ["~/Projects", "~/Developer"]

[keymaps]
quit = "q"
help = "?"
search = "/"
add_project = "a"
create_project = "c"
remove_project = "d"
workspace = "enter"
workspace_alt = "t"
local_preview = "b"
deploy_preview = "B"
editor = "o"
lazygit = "g"
edit_deploy = "u"
config = "e"
theme = "T"
doctor = "D"
kill_session = "x"
stop_dev = "X"
reload = "r"
left = "h"
right = "l"
down = "j"
up = "k"
```

## Homebrew Tap

RunDeck uses a separate Homebrew tap repository:

```txt
IntScription/homebrew-rundeck
```

That repository contains the Homebrew formula used by:

```bash
brew install IntScription/rundeck/rundeck
```

The main repository contains the Rust source code. The Homebrew tap only contains the install recipe.

## Themes

RunDeck supports multiple themes.

Open the theme picker:

```txt
T
```

Then press `Enter` to apply.

## Optional Neovim / LazyVim Plugin

RunDeck includes an optional Neovim helper plugin.

Folder:

```txt
nvim/lua/rundeck.lua
```

LazyVim setup:

```lua
return {
  dir = "~/Projects/Personal/rundeck/nvim",
  name = "rundeck.nvim",
  lazy = false,
  config = function()
    require("rundeck").setup({
      keymaps = {
        open = "<leader>rd",
        add = "<leader>ra",
        create = "<leader>rc",
        config = "<leader>re",
      },
    })
  end,
}
```

Commands:

```vim
:Rundeck
:RundeckAdd
:RundeckCreate
:RundeckConfig
```

Default keymaps:

| Keymap | Action |
|---|---|
| `<leader>rd` | Open RunDeck dashboard |
| `<leader>ra` | Add current Neovim project to RunDeck |
| `<leader>rc` | Open create project helper |
| `<leader>re` | Edit RunDeck config |
