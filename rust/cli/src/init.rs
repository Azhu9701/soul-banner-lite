use std::fs;
use std::path::Path;

pub struct InitArgs {
    pub project_name: String,
    pub domain: String,
    pub port: u16,
    pub frontend_port: u16,
    pub skip_frontend: bool,
}

pub fn run(args: InitArgs) -> Result<(), Box<dyn std::error::Error>> {
    let project_dir = Path::new(&args.project_name);

    if project_dir.exists() {
        eprintln!("Error: directory '{}' already exists", args.project_name);
        std::process::exit(1);
    }

    create_dirs(project_dir, args.skip_frontend)?;
    write_config(project_dir, &args)?;
    write_cargo_toml(project_dir)?;
    write_main_rs(project_dir, &args)?;
    write_env_example(project_dir)?;

    if !args.skip_frontend {
        write_nextjs_skeleton(project_dir, &args)?;
    }

    write_readme(project_dir, &args)?;

    println!("Project '{}' created successfully!", args.project_name);
    println!();
    println!("Next steps:");
    println!("  cd {}", args.project_name);
    println!("  cp .env.example .env   # edit .env with your API keys");
    println!("  cd rust/agent-app && cargo run  # start API (port {})", args.port);
    if !args.skip_frontend {
        println!("  cd nextjs && pnpm dev   # start frontend (port {})", args.frontend_port);
    }

    Ok(())
}

fn create_dirs(project_dir: &Path, skip_frontend: bool) -> Result<(), Box<dyn std::error::Error>> {
    let dirs = vec![
        "config",
        "data/agents",
        "data/knowledge",
        "rust/agent-app/src/agents",
        "rust/agent-app/src/modes",
        "rust/agent-app/src/tools",
    ];

    let frontend_dirs: Vec<&str> = if !skip_frontend {
        vec!["nextjs/app", "nextjs/components", "nextjs/hooks", "nextjs/lib", "nextjs/public"]
    } else {
        vec![]
    };

    for d in dirs.iter().chain(frontend_dirs.iter()) {
        fs::create_dir_all(project_dir.join(d))?;
    }
    Ok(())
}

fn write_config(project_dir: &Path, args: &InitArgs) -> Result<(), Box<dyn std::error::Error>> {
    let default_yaml = format!(
        "# Server configuration\n\
         server_host: \"127.0.0.1\"\n\
         server_port: {port}\n\
         web_port: {frontend_port}\n\
         \n\
         # Data paths\n\
         data_dir: \"./data\"\n\
         agents_dir: \"./data/agents\"\n\
         archive_dir: \"./data/archive\"\n\
         db_path: \"./data/app.db\"\n\
         \n\
         # Rate limiting\n\
         rate_limit:\n\
           enabled: true\n\
           requests_per_second: 30\n\
           burst_size: 60\n\
         \n\
         # CORS\n\
         cors_origins:\n\
           - \"http://localhost:{frontend_port}\"\n",
        port = args.port,
        frontend_port = args.frontend_port,
    );
    fs::write(project_dir.join("config/default.yaml"), default_yaml)?;

    let domain_yaml = match args.domain.as_str() {
        "philosophy" => include_str!("../../../meta/templates/domains/philosophy/domain.yaml"),
        "legal" => include_str!("../../../meta/templates/domains/legal/domain.yaml"),
        "labor" => include_str!("../../../meta/templates/domains/labor/domain.yaml"),
        _ => include_str!("../../../meta/templates/base/domain.yaml.tmpl"),
    };
    fs::write(project_dir.join("config/domain.yaml"), domain_yaml)?;

    let sample_agent = r#"---
name: "assistant"
title: "通用助手"
description: "默认助手Agent"
model: "deepseek-chat"
tools: []
trigger_keywords: ["帮助", "问题"]
system_prompt: |
  你是一个通用助手，请简洁清晰地回答用户问题。
---
"#;
    fs::write(project_dir.join("data/agents/assistant.md"), sample_agent)?;

    Ok(())
}

fn write_cargo_toml(project_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let app_toml = r#"[package]
name = "agent-app"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }

# Framework crates (local path during Phase 1)
foundation = { path = "../../../rust/foundation" }
ai-gateway = { path = "../../../rust/ai-gateway" }
registry = { path = "../../../rust/registry" }
possession = { path = "../../../rust/possession" }
archive = { path = "../../../rust/archive" }
api = { path = "../../../rust/api" }
"#;
    fs::create_dir_all(project_dir.join("rust/agent-app/src"))?;
    fs::write(project_dir.join("rust/agent-app/Cargo.toml"), app_toml)?;
    Ok(())
}

fn write_main_rs(project_dir: &Path, args: &InitArgs) -> Result<(), Box<dyn std::error::Error>> {
    let main_rs = format!(
        r#"use std::sync::Arc;
use foundation::Config;
use api::state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let config = Config::load("config/default.yaml")?;
    let state = AppState::new(config).await?;

    let app = api::routes::create_router(Arc::new(state));

    let addr = format!("127.0.0.1:{port}");
    tracing::info!("API server listening on {{}}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}}
"#,
        port = args.port,
    );

    fs::write(project_dir.join("rust/agent-app/src/main.rs"), main_rs)?;
    Ok(())
}

fn write_env_example(project_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let env = r#"# AI Provider API Keys (fill at least one)
OPENAI_API_KEY=sk-xxxxx
CLAUDE_API_KEY=sk-ant-xxxxx
DEEPSEEK_API_KEY=sk-xxxxx

# Optional: Local model
LMSTUDIO_URL=http://localhost:1234/v1
LMSTUDIO_MODEL=qwen/qwen3.6-27b

# Optional: API auth token
API_TOKEN=your-secret-token
"#;
    fs::write(project_dir.join(".env.example"), env)?;
    Ok(())
}

fn write_readme(project_dir: &Path, args: &InitArgs) -> Result<(), Box<dyn std::error::Error>> {
    let readme = format!(
        "# {name}\n\n\
         Generated with `snake init --domain {domain}`.\n\n\
         ## Quick Start\n\n\
         ```bash\n\
         # 1. Configure API keys\n\
         cp .env.example .env\n\
         # Edit .env with your AI provider keys\n\n\
         # 2. Start API server (port {port})\n\
         cd rust/agent-app && cargo run\n\n\
         # 3. Start frontend (port {frontend_port})\n\
         cd nextjs && pnpm dev\n\
         ```\n\n\
         See `meta/docs/` for framework documentation.\n",
        name = args.project_name,
        domain = args.domain,
        port = args.port,
        frontend_port = args.frontend_port,
    );
    fs::write(project_dir.join("README.md"), readme)?;
    Ok(())
}

fn write_nextjs_skeleton(project_dir: &Path, args: &InitArgs) -> Result<(), Box<dyn std::error::Error>> {
    let pkg_json = format!(
        r#"{{
  "name": "{name}-frontend",
  "version": "0.1.0",
  "private": true,
  "scripts": {{
    "dev": "next dev -p {port}",
    "build": "next build",
    "start": "next start"
  }},
  "dependencies": {{
    "next": "^16",
    "react": "^19",
    "react-dom": "^19",
    "lucide-react": "^1",
    "tailwindcss": "^4",
    "@tailwindcss/postcss": "^4"
  }}
}}
"#,
        name = args.project_name,
        port = args.frontend_port,
    );
    fs::create_dir_all(project_dir.join("nextjs"))?;
    fs::write(project_dir.join("nextjs/package.json"), pkg_json)?;

    let layout_tsx = r#"export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="zh">
      <body>{children}</body>
    </html>
  );
}
"#;
    fs::create_dir_all(project_dir.join("nextjs/app"))?;
    fs::write(project_dir.join("nextjs/app/layout.tsx"), layout_tsx)?;

    let page_tsx = r#"export default function Home() {
  return (
    <main className="p-8">
      <h1 className="text-2xl font-bold">Agent System Ready</h1>
      <p className="mt-2 text-gray-600">Your multi-agent reasoning system is running.</p>
    </main>
  );
}
"#;
    fs::write(project_dir.join("nextjs/app/page.tsx"), page_tsx)?;

    Ok(())
}
