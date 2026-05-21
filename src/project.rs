use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
    pub port: Option<u16>,

    #[serde(default)]
    pub deploy_url: Option<String>,

    #[serde(default)]
    pub dev_command: Option<String>,

    #[serde(default)]
    pub last_opened: Option<u64>,
}

impl Project {
    pub fn slug_name(value: &str) -> String {
        slugify(value)
    }

    pub fn package_root(&self) -> PathBuf {
        self.primary_package_root()
            .unwrap_or_else(|| self.path.clone())
    }

    fn primary_package_root(&self) -> Option<PathBuf> {
        if self.path.join("package.json").exists() {
            return Some(self.path.clone());
        }

        let preferred_dirs = [
            "web",
            "frontend",
            "client",
            "site",
            "app",
            "apps/web",
            "apps/app",
            "packages/web",
            "desktop",
            "mobile",
            "apps/mobile",
            "packages/mobile",
        ];

        for dir in preferred_dirs {
            let candidate = self.path.join(dir);

            if candidate.join("package.json").exists() {
                return Some(candidate);
            }
        }

        self.package_roots().into_iter().next()
    }

    fn package_roots(&self) -> Vec<PathBuf> {
        let mut roots = BTreeSet::new();

        if self.path.join("package.json").exists() {
            roots.insert(self.path.clone());
        }

        let common_dirs = [
            "web",
            "mobile",
            "frontend",
            "client",
            "site",
            "app",
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
            "packages/api",
        ];

        for dir in common_dirs {
            let candidate = self.path.join(dir);

            if candidate.join("package.json").exists() {
                roots.insert(candidate);
            }
        }

        collect_package_roots_shallow(&self.path, 3, &mut roots);

        roots.into_iter().collect()
    }

    pub fn package_scripts(&self) -> BTreeMap<String, String> {
        let package_json_path = self.package_root().join("package.json");

        if !package_json_path.exists() {
            return BTreeMap::new();
        }

        let Ok(content) = fs::read_to_string(package_json_path) else {
            return BTreeMap::new();
        };

        let Ok(json) = serde_json::from_str::<Value>(&content) else {
            return BTreeMap::new();
        };

        let Some(scripts) = json.get("scripts").and_then(|value| value.as_object()) else {
            return BTreeMap::new();
        };

        scripts
            .iter()
            .filter_map(|(key, value)| {
                value
                    .as_str()
                    .map(|script| (key.to_string(), script.to_string()))
            })
            .collect()
    }

    pub fn package_manager(&self) -> &'static str {
        let root = self.package_root();

        if !root.join("package.json").exists() {
            return "none";
        }

        self.package_manager_for(&root)
    }

    fn package_manager_for(&self, root: &Path) -> &'static str {
        if root.join("pnpm-lock.yaml").exists() || self.path.join("pnpm-lock.yaml").exists() {
            "pnpm"
        } else if root.join("yarn.lock").exists() || self.path.join("yarn.lock").exists() {
            "yarn"
        } else if root.join("bun.lockb").exists()
            || root.join("bun.lock").exists()
            || self.path.join("bun.lockb").exists()
            || self.path.join("bun.lock").exists()
        {
            "bun"
        } else {
            "npm"
        }
    }

    pub fn effective_port(&self) -> Option<u16> {
        self.port.or_else(|| detect_project_port(&self.path))
    }

    pub fn localhost_url(&self) -> Option<String> {
        self.effective_port()
            .map(|port| format!("http://localhost:{port}"))
    }

    pub fn deployed_url(&self) -> Option<String> {
        self.deploy_url
            .clone()
            .or_else(|| self.package_json_string("homepage"))
    }

    pub fn urls(&self) -> Vec<(String, String)> {
        let mut urls = Vec::new();

        if let Some(localhost) = self.localhost_url() {
            urls.push(("Local".to_string(), localhost));
        }

        if let Some(url) = self.deployed_url() {
            urls.push(("Deploy".to_string(), url));
        }

        if let Some(repository) = self.repository_url() {
            urls.push(("Repo".to_string(), repository));
        }

        if self.has_vercel_marker() && self.deploy_url.is_none() {
            urls.push((
                "Deploy".to_string(),
                "Vercel detected, add deploy URL with u".to_string(),
            ));
        }

        urls
    }

    fn has_vercel_marker(&self) -> bool {
        if self.path.join(".vercel").exists() {
            return true;
        }

        self.package_roots()
            .iter()
            .any(|root| root.join(".vercel").exists())
    }

    pub fn dev_server_command(&self) -> Option<String> {
        if let Some(command) = &self.dev_command {
            return Some(command.clone());
        }

        let scripts = self.package_scripts();
        let package_root = self.package_root();
        let package_manager = self.package_manager_for(&package_root);

        let script_command = |script: &str| -> Option<String> {
            if !scripts.contains_key(script) {
                return None;
            }

            match package_manager {
                "pnpm" => Some(format!("pnpm {script}")),
                "yarn" => Some(format!("yarn {script}")),
                "bun" => Some(format!("bun run {script}")),
                "npm" => Some(format!("npm run {script}")),
                _ => None,
            }
        };

        if let Some(command) = script_command("dev") {
            return Some(command);
        }

        if let Some(command) = script_command("start") {
            return Some(command);
        }

        if let Some(command) = script_command("preview") {
            return Some(command);
        }

        let primary_stack = self.stack_items_for_root(&package_root);

        if primary_stack.iter().any(|item| item == "Next.js") {
            return match package_manager {
                "pnpm" => Some("pnpm exec next dev".to_string()),
                "yarn" => Some("yarn next dev".to_string()),
                "bun" => Some("bunx next dev".to_string()),
                "npm" => Some("npx next dev".to_string()),
                _ => None,
            };
        }

        if primary_stack.iter().any(|item| item == "Vite") {
            return match package_manager {
                "pnpm" => Some("pnpm exec vite".to_string()),
                "yarn" => Some("yarn vite".to_string()),
                "bun" => Some("bunx vite".to_string()),
                "npm" => Some("npx vite".to_string()),
                _ => None,
            };
        }

        if primary_stack.iter().any(|item| item == "Expo") {
            return match package_manager {
                "pnpm" => Some("pnpm exec expo start".to_string()),
                "yarn" => Some("yarn expo start".to_string()),
                "bun" => Some("bunx expo start".to_string()),
                "npm" => Some("npx expo start".to_string()),
                _ => None,
            };
        }

        None
    }

    pub fn stack_summary(&self) -> String {
        let stack = self.stack_items();

        if stack.is_empty() {
            if !self.package_roots().is_empty() {
                "Node.js workspace".to_string()
            } else if self.path.join("Cargo.toml").exists() {
                "Rust".to_string()
            } else {
                "No known stack detected yet".to_string()
            }
        } else {
            stack.join(" · ")
        }
    }

    pub fn stack_items(&self) -> Vec<String> {
        let mut stack = Vec::new();

        for root in self.package_roots() {
            for item in self.stack_items_for_root(&root) {
                push_unique(&mut stack, &item);
            }
        }

        self.detect_workspace_stack(&mut stack);

        stack
    }

    fn stack_items_for_root(&self, root: &Path) -> Vec<String> {
        let mut stack = Vec::new();

        let Some(json) = read_package_json(root) else {
            return stack;
        };

        let deps = json
            .get("dependencies")
            .and_then(|value| value.as_object())
            .cloned()
            .unwrap_or_default();

        let dev_deps = json
            .get("devDependencies")
            .and_then(|value| value.as_object())
            .cloned()
            .unwrap_or_default();

        let has = |name: &str| deps.contains_key(name) || dev_deps.contains_key(name);

        if has("next") {
            push_unique(&mut stack, "Next.js");
        }

        if has("react") {
            push_unique(&mut stack, "React");
        }

        if has("react-native") {
            push_unique(&mut stack, "React Native");
        }

        if has("expo") {
            push_unique(&mut stack, "Expo");
            push_unique(&mut stack, "React Native");
        }

        if has("typescript") || root.join("tsconfig.json").exists() {
            push_unique(&mut stack, "TypeScript");
        }

        if has("@supabase/supabase-js") {
            push_unique(&mut stack, "Supabase");
        }

        if has("tailwindcss")
            || root.join("tailwind.config.js").exists()
            || root.join("tailwind.config.ts").exists()
            || root.join("postcss.config.js").exists()
            || root.join("postcss.config.mjs").exists()
        {
            push_unique(&mut stack, "Tailwind");
        }

        if has("vite")
            || root.join("vite.config.ts").exists()
            || root.join("vite.config.js").exists()
            || root.join("vite.config.mjs").exists()
        {
            push_unique(&mut stack, "Vite");
        }

        if has("@tauri-apps/api") || root.join("src-tauri").exists() {
            push_unique(&mut stack, "Tauri");
        }

        if has("stripe") || has("@stripe/stripe-js") {
            push_unique(&mut stack, "Stripe");
        }

        if has("zod") {
            push_unique(&mut stack, "Zod");
        }

        if has("@tanstack/react-query") {
            push_unique(&mut stack, "TanStack Query");
        }

        if has("zustand") {
            push_unique(&mut stack, "Zustand");
        }

        if has("prisma") || has("@prisma/client") || root.join("prisma").exists() {
            push_unique(&mut stack, "Prisma");
        }

        if has("drizzle-orm") {
            push_unique(&mut stack, "Drizzle");
        }

        if root.join("app").exists() && stack.iter().any(|item| item == "Next.js") {
            push_unique(&mut stack, "App Router");
        }

        if root.join("pages").exists() && stack.iter().any(|item| item == "Next.js") {
            push_unique(&mut stack, "Pages Router");
        }

        stack
    }

    fn detect_workspace_stack(&self, stack: &mut Vec<String>) {
        if self.path.join("supabase").exists()
            || self
                .package_roots()
                .iter()
                .any(|root| root.join("supabase").exists())
        {
            push_unique(stack, "Supabase");
        }

        if self.path.join("Cargo.toml").exists()
            || self
                .package_roots()
                .iter()
                .any(|root| root.join("Cargo.toml").exists())
            || self.path.join("src-tauri").join("Cargo.toml").exists()
        {
            push_unique(stack, "Rust");
        }

        if self.path.join("src-tauri").exists()
            || self
                .package_roots()
                .iter()
                .any(|root| root.join("src-tauri").exists())
        {
            push_unique(stack, "Tauri");
        }

        if self.path.join("go.mod").exists() {
            push_unique(stack, "Go");
        }

        if self.path.join("pyproject.toml").exists()
            || self.path.join("requirements.txt").exists()
            || self.path.join("Pipfile").exists()
        {
            push_unique(stack, "Python");
        }

        if self.path.join("pubspec.yaml").exists() {
            push_unique(stack, "Flutter");
        }

        if self.path.join("docker-compose.yml").exists()
            || self.path.join("docker-compose.yaml").exists()
            || self.path.join("Dockerfile").exists()
        {
            push_unique(stack, "Docker");
        }
    }

    pub fn tmux_session_name(&self) -> String {
        slugify(&self.name)
    }

    fn package_json(&self) -> Option<Value> {
        read_package_json(&self.package_root())
    }

    fn package_json_string(&self, key: &str) -> Option<String> {
        for root in self.package_roots() {
            let Some(json) = read_package_json(&root) else {
                continue;
            };

            if let Some(value) = json.get(key).and_then(|value| value.as_str()) {
                return Some(value.to_string());
            }
        }

        self.package_json()?
            .get(key)?
            .as_str()
            .map(|value| value.to_string())
    }

    fn repository_url(&self) -> Option<String> {
        for root in self.package_roots() {
            let Some(json) = read_package_json(&root) else {
                continue;
            };

            let Some(repository) = json.get("repository") else {
                continue;
            };

            if let Some(url) = repository.as_str() {
                return Some(url.to_string());
            }

            if let Some(url) = repository.get("url").and_then(|value| value.as_str()) {
                return Some(url.to_string());
            }
        }

        None
    }
}

pub fn slugify(value: &str) -> String {
    let mut output = String::new();
    let mut last_dash = false;

    for ch in value.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch);
            last_dash = false;
        } else if !last_dash {
            output.push('-');
            last_dash = true;
        }
    }

    let output = output.trim_matches('-').to_string();

    if output.is_empty() {
        "rundeck-project".to_string()
    } else {
        output
    }
}

fn read_package_json(root: &Path) -> Option<Value> {
    let content = fs::read_to_string(root.join("package.json")).ok()?;
    serde_json::from_str::<Value>(&content).ok()
}

fn push_unique(stack: &mut Vec<String>, item: &str) {
    if !stack.iter().any(|existing| existing == item) {
        stack.push(item.to_string());
    }
}

fn collect_package_roots_shallow(path: &Path, depth: usize, roots: &mut BTreeSet<PathBuf>) {
    if depth == 0 || !path.is_dir() || should_ignore_dir(path) {
        return;
    }

    if path.join("package.json").exists() {
        roots.insert(path.to_path_buf());
    }

    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let entry_path = entry.path();

        if entry_path.is_dir() {
            collect_package_roots_shallow(&entry_path, depth - 1, roots);
        }
    }
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
        "ios",
        "android",
    ];

    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| ignored.contains(&name))
        .unwrap_or(false)
}

fn detect_project_port(path: &Path) -> Option<u16> {
    let mut roots = BTreeSet::new();

    if path.join("package.json").exists() {
        roots.insert(path.to_path_buf());
    }

    let preferred_dirs = [
        "web",
        "frontend",
        "client",
        "site",
        "apps/web",
        "packages/web",
        "app",
    ];

    for dir in preferred_dirs {
        let candidate = path.join(dir);

        if candidate.join("package.json").exists() {
            roots.insert(candidate);
        }
    }

    collect_package_roots_shallow(path, 3, &mut roots);

    for root in roots {
        let Ok(content) = fs::read_to_string(root.join("package.json")) else {
            continue;
        };

        if let Some(port) = detect_port_from_package_text(&content) {
            return Some(port);
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
