<h1 align="center">RunDeck</h1>

<p align="center">
  <strong>A fast terminal dashboard for launching, managing, and jumping between personal development projects.</strong>
</p>

<p align="center">
  RunDeck brings your projects, tmux workspaces, Neovim, lazygit, local previews, deploy links, and project metadata into one clean keyboard-driven terminal UI.
</p>

<p align="center">
  <img alt="GitHub release" src="https://img.shields.io/github/v/release/IntScription/rundeck?style=for-the-badge" />
  <img alt="CI status" src="https://img.shields.io/github/actions/workflow/status/IntScription/rundeck/ci.yml?branch=main&style=for-the-badge&label=CI" />
  <img alt="License" src="https://img.shields.io/github/license/IntScription/rundeck?style=for-the-badge" />
  <img alt="Built with Rust" src="https://img.shields.io/badge/Built%20with-Rust-orange?style=for-the-badge&logo=rust" />
  <img alt="Terminal first" src="https://img.shields.io/badge/Terminal-first-7aa2f7?style=for-the-badge" />
</p>

<p align="center">
  <a href="#rundeck-installation">Installation</a>
  ·
  <a href="#rundeck-usage">Usage</a>
  ·
  <a href="#rundeck-themes">Themes</a>
  ·
  <a href="#rundeck-configuration">Configuration</a>
  ·
  <a href="#rundeck-troubleshooting">Troubleshooting</a>
</p>

<p align="center">
  <img src="./assets/catppuccin-mocha.png" alt="RunDeck Catppuccin Mocha Preview" width="100%" />
</p>

<a name="rundeck-contents"></a>

## Contents

- [Why RunDeck](#rundeck-why-rundeck)
- [Features](#rundeck-features)
- [Installation](#rundeck-installation)
  - [Quick install](#rundeck-quick-install)
  - [Install by platform](#rundeck-install-by-platform)
  - [Updating](#rundeck-updating)
  - [Uninstalling](#rundeck-uninstalling)
- [Requirements](#rundeck-requirements)
- [LazyVim setup](#rundeck-lazyvim-setup)
- [Recommended workflow](#rundeck-recommended-workflow)
- [Usage](#rundeck-usage)
- [Dashboard keymaps](#rundeck-dashboard-keymaps)
- [tmux workspace](#rundeck-tmux-workspace)
- [Local preview](#rundeck-local-preview)
- [Supported project types](#rundeck-supported-project-types)
- [Monorepo and workspace support](#rundeck-monorepo-and-workspace-support)
- [Configuration](#rundeck-configuration)
- [Homebrew tap](#rundeck-homebrew-tap)
- [Themes](#rundeck-themes)
- [Optional Neovim and LazyVim plugin](#rundeck-optional-neovim-and-lazyvim-plugin)
- [Docker](#rundeck-docker)
- [Kubernetes](#rundeck-kubernetes)
- [Troubleshooting](#rundeck-troubleshooting)
- [License](#rundeck-license)

<a name="rundeck-why-rundeck"></a>

## Why RunDeck?

Most developers keep switching between:

- terminal folders
- tmux sessions
- Neovim
- lazygit
- localhost URLs
- deploy links
- project notes/configs

RunDeck turns that messy workflow into one fast terminal dashboard.

Instead of remembering where every project lives, which port it runs on, or which command starts it, RunDeck gives you a single keyboard-first place to launch and manage everything.

<a name="rundeck-features"></a>

## Features

- Fast terminal dashboard built in Rust
- Project launcher with tmux + Neovim workspace support
- Automatic stack detection
- Local preview launcher
- Deploy URL launcher
- lazygit shortcut
- Add existing projects with fzf
- Create new projects from the dashboard
- Remove projects from RunDeck without deleting folders
- Auto-removes missing projects when folders are deleted
- Configurable keymaps
- Multiple terminal-friendly themes
- Optional Neovim/LazyVim companion plugin
- Works great with custom dotfiles, tmux layouts, and terminal-first workflows

<a name="rundeck-installation"></a>

## Installation

<a name="rundeck-quick-install"></a>

### Quick install

```bash
curl -fsSL https://raw.githubusercontent.com/IntScription/rundeck/main/scripts/install.sh | bash
```

Then run:

```bash
rundeck
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

<a name="rundeck-install-by-platform"></a>

### Install by platform

#### macOS

Using Homebrew:

```bash
brew install IntScription/rundeck/rundeck
```

Or tap first:

```bash
brew tap IntScription/rundeck
brew install rundeck
```

Using the install script:

```bash
curl -fsSL https://raw.githubusercontent.com/IntScription/rundeck/main/scripts/install.sh | bash
```

From source:

```bash
cargo install --git https://github.com/IntScription/rundeck --force
```

#### Linux

Using the universal install script:

```bash
curl -fsSL https://raw.githubusercontent.com/IntScription/rundeck/main/scripts/install.sh | bash
```

Using Homebrew on Linux:

```bash
brew install IntScription/rundeck/rundeck
```

From source:

```bash
cargo install --git https://github.com/IntScription/rundeck --force
```

From a local clone:

```bash
git clone https://github.com/IntScription/rundeck.git
cd rundeck
cargo install --path . --force
```

#### Linux package-manager notes

RunDeck can be distributed to Linux users in multiple package formats. Use the package that matches your distro once release artifacts are available.

Ubuntu / Debian:

```bash
sudo apt install ./rundeck-linux-amd64.deb
```

Fedora / RHEL:

```bash
sudo dnf install ./rundeck-linux-x86_64.rpm
```

Arch Linux / Manjaro:

```bash
sudo pacman -U rundeck-linux-x86_64.pkg.tar.zst
```

If an AUR package is published later:

```bash
yay -S rundeck
```

#### Windows

RunDeck is designed around terminal tools like tmux, Neovim, lazygit, and local project folders. On Windows, WSL2 is recommended for the best experience.

#### Windows with WSL2 recommended

Inside your WSL terminal:

```bash
curl -fsSL https://raw.githubusercontent.com/IntScription/rundeck/main/scripts/install.sh | bash
```

Then run:

```bash
rundeck
```

#### Native Windows from source

Install Rust, Git, and Neovim, then run:

```powershell
cargo install --git https://github.com/IntScription/rundeck --force
```

If you add a PowerShell installer later, you can expose it like this:

```powershell
irm https://raw.githubusercontent.com/IntScription/rundeck/main/scripts/install.ps1 | iex
```

<a name="rundeck-updating"></a>

### Updating

Homebrew:

```bash
brew update
brew upgrade rundeck
```

Source install:

```bash
cargo install --git https://github.com/IntScription/rundeck --force
```

Local clone:

```bash
cd rundeck
git pull
cargo install --path . --force
```

Docker:

```bash
docker pull ghcr.io/intscription/rundeck:latest
```

<a name="rundeck-uninstalling"></a>

### Uninstalling

Homebrew:

```bash
brew uninstall rundeck
brew untap IntScription/rundeck
```

Cargo/source install:

```bash
cargo uninstall rundeck
```

Remove RunDeck config:

```bash
rm -rf ~/.config/rundeck
```

Remove local data only if you know you no longer need it:

```bash
rm -rf ~/.local/share/rundeck
```

<a name="rundeck-requirements"></a>

## Requirements

RunDeck works best with:

- tmux
- git
- fzf
- lazygit
- Neovim
- LazyVim optional, but recommended for the full terminal IDE workflow
- Rust/Cargo only required for source installs

Check your setup:

```bash
rundeck doctor
```

### macOS tools

```bash
brew install tmux lazygit fzf neovim ripgrep fd
```

LazyVim is not a separate Homebrew package. It is a Neovim config setup. Install it using the [LazyVim setup](#rundeck-lazyvim-setup) section below.

### Linux tools

Ubuntu / Debian:

```bash
sudo apt update
sudo apt install -y git tmux fzf neovim ripgrep fd-find curl build-essential
```

Optional `fd` alias for Ubuntu/Debian, because the binary may be named `fdfind`:

```bash
mkdir -p ~/.local/bin
ln -sf "$(command -v fdfind)" ~/.local/bin/fd
```

Fedora:

```bash
sudo dnf install -y git tmux fzf neovim ripgrep fd-find curl gcc gcc-c++ make
```

Arch Linux / Manjaro:

```bash
sudo pacman -S git tmux fzf neovim ripgrep fd curl base-devel
```

Install lazygit using your distro package manager, Homebrew on Linux, or the official lazygit release package.

### Windows tools

For the smoothest setup, install these inside WSL2 using the Linux commands above.

For native Windows, use Windows Terminal plus your preferred package manager:

```powershell
winget install Git.Git Neovim.Neovim Rustlang.Rustup
```

<a name="rundeck-lazyvim-setup"></a>

## LazyVim setup

LazyVim is optional, but it pairs well with RunDeck because RunDeck can launch project workspaces directly into Neovim.

### Install LazyVim starter

Back up any existing Neovim config first:

```bash
mv ~/.config/nvim{,.bak}
mv ~/.local/share/nvim{,.bak}
mv ~/.local/state/nvim{,.bak}
mv ~/.cache/nvim{,.bak}
```

Clone the LazyVim starter:

```bash
git clone https://github.com/LazyVim/starter ~/.config/nvim
rm -rf ~/.config/nvim/.git
nvim
```

After opening Neovim, run:

```vim
:LazyHealth
```

### Use my dotfiles

If you want the same terminal-first setup with LazyVim, tmux, Alacritty, and related configs, you can use my dotfiles:

```bash
git clone https://github.com/IntScription/dotfiles ~/.dotfiles
cd ~/.dotfiles
./install.sh
```

Or manually stow only the configs you want:

```bash
cd ~/.dotfiles
stow nvim tmux alacritty
```

The dotfiles repo is useful if you want a ready-to-use LazyVim and tmux workflow that matches the way RunDeck is intended to be used.

<a name="rundeck-recommended-workflow"></a>

## Recommended workflow

| Step | Action |
|---:|---|
| 1 | Open RunDeck with `rundeck` |
| 2 | Press `a` to add an existing project |
| 3 | Press `Enter` to open the selected project workspace |
| 4 | Use the top pane for Neovim and the bottom pane for shell commands |
| 5 | Press `g` when you need lazygit |
| 6 | Press `b` to start/open the local preview |
| 7 | Use `rundeck back` to return to the dashboard |

This is the workflow RunDeck is built for: open a project, code in Neovim, manage Git with lazygit, preview locally, and jump back to your project list without breaking terminal flow.

<a name="rundeck-usage"></a>

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

<a name="rundeck-dashboard-keymaps"></a>

## Dashboard keymaps

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

<a name="rundeck-tmux-workspace"></a>

## tmux workspace

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

<a name="rundeck-local-preview"></a>

## Local preview

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

<a name="rundeck-supported-project-types"></a>

## Supported project types

RunDeck can detect common project stacks including:

| Category | Examples |
|---|---|
| Frontend | React, Next.js, Vite, Tailwind |
| Mobile | Expo, React Native |
| Backend | Node.js, Python, Go, Rust |
| Infra / services | Supabase, Docker, Kubernetes |
| Desktop / native | Tauri, Rust |
| Workspaces | `web`, `mobile`, `apps`, `packages`, monorepos |

The stack detector reads common project markers like `package.json`, framework configs, workspace folders, Rust manifests, Supabase folders, and other development files.

<a name="rundeck-monorepo-and-workspace-support"></a>

## Monorepo and workspace support

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

Example detected stack:

```txt
Next.js · React · TypeScript · Tailwind · Expo · React Native · Supabase
```

<a name="rundeck-configuration"></a>

## Configuration

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

<a name="rundeck-homebrew-tap"></a>

## Homebrew tap

RunDeck uses a separate Homebrew tap repository:

```txt
IntScription/homebrew-rundeck
```

That repository contains the Homebrew formula used by:

```bash
brew install IntScription/rundeck/rundeck
```

Or tap first:

```bash
brew tap IntScription/rundeck
brew install rundeck
```

The main repository contains the Rust source code. The Homebrew tap only contains the install recipe.

<a name="rundeck-themes"></a>

## Themes

RunDeck ships with multiple terminal-friendly themes. The main screenshot at the top of this README uses the **Catppuccin Mocha** theme.

Open the theme picker from the dashboard:

```txt
T
```

Then press `Enter` to apply the selected theme, or `q` to close the picker.

You can also set a theme manually in `~/.config/rundeck/config.toml`:

```toml
theme = "catppuccin-mocha"
```

Available theme values:

```txt
catppuccin-mocha
tokyo-night
kanagawa-wave
gruvbox-dark
rose-pine
nord
dracula
```

### Theme gallery

The gallery expects the screenshots to be stored in the `assets/` folder with these filenames:

```txt
catppuccin-mocha.png
tokyonight.png
kanagawa-wave.png
gruvbox-dark.png
rose-pine.png
nord.png
dracula.png
```

<table>
  <tr>
    <td width="50%" align="center" valign="top">
      <strong>Catppuccin Mocha</strong><br />
      <img src="./assets/catppuccin-mocha.png" alt="RunDeck Catppuccin Mocha theme" width="100%" />
    </td>
    <td width="50%" align="center" valign="top">
      <strong>Tokyo Night</strong><br />
      <img src="./assets/tokyonight.png" alt="RunDeck Tokyo Night theme" width="100%" />
    </td>
  </tr>
  <tr>
    <td width="50%" align="center" valign="top">
      <strong>Kanagawa Wave</strong><br />
      <img src="./assets/kanagawa-wave.png" alt="RunDeck Kanagawa Wave theme" width="100%" />
    </td>
    <td width="50%" align="center" valign="top">
      <strong>Gruvbox Dark</strong><br />
      <img src="./assets/gruvbox-dark.png" alt="RunDeck Gruvbox Dark theme" width="100%" />
    </td>
  </tr>
  <tr>
    <td width="50%" align="center" valign="top">
      <strong>Rose Pine</strong><br />
      <img src="./assets/rose-pine.png" alt="RunDeck Rose Pine theme" width="100%" />
    </td>
    <td width="50%" align="center" valign="top">
      <strong>Nord</strong><br />
      <img src="./assets/nord.png" alt="RunDeck Nord theme" width="100%" />
    </td>
  </tr>
  <tr>
    <td width="50%" align="center" valign="top">
      <strong>Dracula</strong><br />
      <img src="./assets/dracula.png" alt="RunDeck Dracula theme" width="100%" />
    </td>
    <td width="50%" valign="top"></td>
  </tr>
</table>

<a name="rundeck-optional-neovim-and-lazyvim-plugin"></a>

## Optional Neovim and LazyVim plugin

RunDeck includes an optional Neovim helper plugin.

Folder:

```txt
nvim/lua/rundeck.lua
```

LazyVim plugin setup:

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

<a name="rundeck-docker"></a>

## Docker

Docker support is mainly for development, testing, and container users. RunDeck is designed to run directly on the host because it integrates with tmux, Neovim, local folders, browser URLs, and user config.

Pull from GitHub Container Registry if the image is published:

```bash
docker pull ghcr.io/intscription/rundeck:latest
```

Run doctor:

```bash
docker run --rm ghcr.io/intscription/rundeck:latest doctor
```

Build locally:

```bash
docker build -t rundeck:local .
```

Run doctor locally:

```bash
docker run --rm rundeck:local doctor
```

Open a container shell:

```bash
docker compose run --rm shell
```

<a name="rundeck-kubernetes"></a>

## Kubernetes

Kubernetes support is provided as example manifests for running RunDeck as a toolbox/job container.

Apply:

```bash
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/job.yaml
```

View doctor output:

```bash
kubectl logs -n rundeck job/rundeck-doctor
```

Run toolbox pod:

```bash
kubectl apply -f k8s/toolbox-pod.yaml
kubectl exec -n rundeck -it rundeck-toolbox -- zsh
```

Clean up:

```bash
kubectl delete namespace rundeck
```

<a name="rundeck-troubleshooting"></a>

## Troubleshooting

### rundeck command not found

Add Cargo/local binaries to your shell path:

```bash
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
```

For zsh:

```bash
echo 'export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### tmux workspace does not open

Check that tmux is installed:

```bash
tmux -V
```

Then run diagnostics:

```bash
rundeck doctor
```

### lazygit does not open

Check that lazygit is installed:

```bash
lazygit --version
```

### Neovim does not open

Check your editor:

```bash
echo $EDITOR
```

Or set the editor in `~/.config/rundeck/config.toml`:

```toml
editor = "nvim"
```

### Local preview opens the wrong port

Set the project port manually:

```toml
[[projects]]
name = "My App"
path = "/Users/me/Projects/my-app"
port = 3000
dev_command = "npm run dev"
```


<a name="rundeck-license"></a>

## License

MIT License.
