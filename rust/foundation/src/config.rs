use config::{Config as ConfigBuilder, Environment, File};
use std::path::{Path, PathBuf};

use crate::domain::DomainProfile;
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct Config {
    pub data_dir: PathBuf,
    pub souls_dir: PathBuf,
    pub souls_internal_dir: Option<PathBuf>,
    pub archive_dir: PathBuf,
    pub db_path: PathBuf,
    pub registry_path: PathBuf,
    pub call_records_path: PathBuf,
    pub server_host: String,
    pub server_port: u16,
    pub web_port: u16,
    pub searxng_url: String,
    pub search_engine: String,
    pub api_token: Option<String>,
    pub cors_origins: Vec<String>,
    /// 领域语义配置——术语、坐标轴、综合模板。默认 = 哲学领域。
    pub domain: DomainProfile,
}

impl Config {
    pub fn load() -> Result<Self> {
        let builder = ConfigBuilder::builder()
            .add_source(File::from(Path::new("config/default.yaml")))
            .add_source(File::from(Path::new("config/local.yaml")).required(false))
            .add_source(Environment::with_prefix("WANMINFAN"));

        let cfg = builder.build()?;

        let data_dir = cfg.get_string("data_dir").unwrap_or_else(|_| "./data".into());

        Ok(Config {
            souls_dir: PathBuf::from(&data_dir).join("souls"),
            souls_internal_dir: std::env::var("WANMINFAN_SOULS_INTERNAL_DIR")
                .ok()
                .map(PathBuf::from)
                .or_else(|| {
                    let default = PathBuf::from(&data_dir).join("souls-internal");
                    if default.exists() { Some(default) } else { None }
                }),
            archive_dir: PathBuf::from(&data_dir).join("archive"),
            db_path: PathBuf::from(&data_dir).join("wanminfan.db"),
            registry_path: PathBuf::from(&data_dir).join("registry.yaml"),
            call_records_path: PathBuf::from(&data_dir).join("call-records.yaml"),
            data_dir: PathBuf::from(data_dir),
            server_host: cfg.get_string("server_host").unwrap_or_else(|_| "127.0.0.1".into()),
            server_port: cfg.get_int("server_port").map(|p| p as u16).unwrap_or(3001),
            web_port: cfg.get_int("web_port").map(|p| p as u16).unwrap_or(3000),
            searxng_url: cfg.get_string("searxng_url").unwrap_or_else(|_| "http://127.0.0.1:8080".into()),
            search_engine: cfg.get_string("search_engine").unwrap_or_else(|_| "bing".into()),
            api_token: cfg.get_string("api_token").ok()
                .or_else(|| std::env::var("WANMINFAN_API_TOKEN").ok())
                .filter(|t| !t.is_empty()),
            cors_origins: cfg
                .get::<Vec<String>>("cors_origins")
                .or_else(|_| {
                    // 兼容旧式逗号分隔字符串写法
                    cfg.get_string("cors_origins")
                        .map(|s| s.split(',').map(|o| o.trim().to_string()).collect())
                })
                .unwrap_or_else(|_| vec!["http://localhost:3000".into()]),
            domain: Self::load_domain(),
        })
    }

    /// 加载领域配置。优先级：config/domain.yaml > 内置默认值。
    /// config/domain.yaml 如果存在，会提供带 {占位符} 的模板；
    /// 加载后调用 render() 进行术语替换。
    /// 失败（文件不存在/解析错误）静默降级到默认值——保证向后兼容。
    fn load_domain() -> DomainProfile {
        let candidates = ["config/domain.yaml", "config/domain.yml"];
        for path in &candidates {
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    match serde_yaml::from_str::<DomainProfile>(&content) {
                        Ok(mut profile) => {
                            tracing::info!("Loaded domain profile from {}", path);
                            // 对从文件加载的模板做术语渲染（内置默认值是已渲染的最终文本，不需要渲染）
                            profile.synthesis_system_prompt = profile.render(&profile.synthesis_system_prompt);
                            profile.collect_system_intro = profile.render(&profile.collect_system_intro);
                            return profile;
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse {}: {} — using default domain", path, e);
                        }
                    }
                }
                Err(_) => continue,
            }
        }
        // 没有配置文件——用内置默认值（哲学领域）
        DomainProfile::default()
    }
}
