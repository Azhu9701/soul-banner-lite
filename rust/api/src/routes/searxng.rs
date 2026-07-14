use std::sync::Arc;

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/search", get(search))
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: String,
    #[serde(default = "default_page")]
    pageno: u32,
    #[serde(default = "default_lang")]
    language: String,
    categories: Option<String>,
}

fn default_page() -> u32 { 1 }
fn default_lang() -> String { "zh".into() }

#[derive(Debug, Serialize)]
struct SearxngResponse {
    query: String,
    number_of_results: u64,
    results: Vec<SearxngResult>,
    suggestions: Vec<String>,
    unresponsive_engines: Vec<Vec<String>>,
    cached: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearxngResult {
    title: String,
    url: String,
    content: String,
    engine: String,
    engines: Vec<String>,
    score: f64,
    category: String,
}

#[derive(Debug, Deserialize)]
struct SearxngRawResponse {
    query: Option<String>,
    number_of_results: Option<u64>,
    results: Option<Vec<SearxngRawResult>>,
    suggestions: Option<Vec<String>>,
    unresponsive_engines: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Deserialize)]
struct SearxngRawResult {
    title: String,
    url: String,
    content: Option<String>,
    engine: String,
    engines: Option<Vec<String>>,
    score: Option<f64>,
    category: Option<String>,
}

async fn search(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearxngResponse>, (axum::http::StatusCode, Json<ApiError>)> {
    let categories = query.categories.as_deref().unwrap_or("");

    // ── Cache lookup ──
    if let Some(cached) = state.searxng_cache.get(&query.q, &query.language, query.pageno, categories) {
        let results: Vec<SearxngResult> = serde_json::from_str(&cached.results_json).unwrap_or_default();
        let suggestions: Vec<String> = serde_json::from_str(&cached.suggestions_json).unwrap_or_default();
        let unresponsive_engines: Vec<Vec<String>> = serde_json::from_str(&cached.unresponsive_engines_json).unwrap_or_default();

        tracing::debug!("SearXNG cache hit: q=\"{}\"", query.q);
        return Ok(Json(SearxngResponse {
            query: query.q,
            number_of_results: cached.number_of_results,
            results,
            suggestions,
            unresponsive_engines,
            cached: true,
        }));
    }

    // ── Cache miss — call SearXNG ──
    let searxng_url = &state.config.searxng_url;
    let mut url = format!("{}/search", searxng_url.trim_end_matches('/'));

    url.push_str(&format!("?format=json&q={}", urlencoding(&query.q)));
    url.push_str(&format!("&pageno={}", query.pageno));
    url.push_str(&format!("&language={}", query.language));
    if let Some(ref cats) = query.categories {
        url.push_str(&format!("&categories={}", urlencoding(cats)));
    }

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| {
            let api_err = ApiError { error: format!("SearXNG 请求失败: {e}") };
            (axum::http::StatusCode::BAD_GATEWAY, Json(api_err))
        })?;

    let status = resp.status();
    let body = resp.text().await.map_err(|e| {
        let api_err = ApiError { error: format!("SearXNG 响应读取失败: {e}") };
        (axum::http::StatusCode::BAD_GATEWAY, Json(api_err))
    })?;

    if !status.is_success() {
        let api_err = ApiError { error: format!("SearXNG 返回错误 ({}): {body}", status.as_u16()) };
        return Err((axum::http::StatusCode::BAD_GATEWAY, Json(api_err)));
    }

    let raw: SearxngRawResponse = serde_json::from_str(&body).map_err(|e| {
        let api_err = ApiError { error: format!("SearXNG 响应解析失败: {e}") };
        (axum::http::StatusCode::BAD_GATEWAY, Json(api_err))
    })?;

    let results: Vec<SearxngResult> = raw.results.unwrap_or_default().into_iter().map(|r| SearxngResult {
        title: r.title,
        url: r.url,
        content: r.content.unwrap_or_default(),
        engine: r.engine,
        engines: r.engines.unwrap_or_default(),
        score: r.score.unwrap_or(0.0),
        category: r.category.unwrap_or_else(|| "general".into()),
    }).collect();

    let suggestions = raw.suggestions.unwrap_or_default();
    let unresponsive_engines = raw.unresponsive_engines.unwrap_or_default();
    let number_of_results = raw.number_of_results.unwrap_or(0);

    // ── Store in cache ──
    let results_json = serde_json::to_string(&results).unwrap_or_default();
    let suggestions_json = serde_json::to_string(&suggestions).unwrap_or_default();
    let unresponsive_json = serde_json::to_string(&unresponsive_engines).unwrap_or_default();
    
    state.searxng_cache.set(
        &query.q,
        &query.language,
        query.pageno,
        categories,
        &results_json,
        &suggestions_json,
        &unresponsive_json,
        number_of_results,
    );

    Ok(Json(SearxngResponse {
        query: raw.query.unwrap_or(query.q),
        number_of_results,
        results,
        suggestions,
        unresponsive_engines,
        cached: false,
    }))
}

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
