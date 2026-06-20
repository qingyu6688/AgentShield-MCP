//! 本地 Web 仪表盘后端。
//!
//! 用 tiny_http 提供只读的 JSON API 并托管构建好的 Vue 前端，复用现有的
//! 审计 / 配置 / 决策记忆能力。实时事件由前端轮询 `/api/events` 实现
//! （刷新间隔 ~1s）。结构上预留 Tauri：后续可由 Tauri 直接复用这些查询函数。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use agentshield_audit::{EventQuery, Format, Report, ReportMeta, SqliteStore};
use agentshield_core::{Config, DecisionMemory};
use serde_json::json;
use tiny_http::{Header, Response, Server};

/// 仪表盘要用到的文件路径。
pub struct DashboardPaths {
    pub db: PathBuf,
    pub config: PathBuf,
    pub decisions: PathBuf,
}

/// 启动仪表盘 HTTP 服务（阻塞）。
pub fn run_dashboard(addr: &str, web_dir: PathBuf, paths: DashboardPaths) -> anyhow::Result<()> {
    let server =
        Server::http(addr).map_err(|e| anyhow::anyhow!("启动仪表盘 HTTP 服务失败：{e}"))?;
    println!("AgentShield 仪表盘已启动：http://{addr}");
    if web_dir.join("index.html").exists() {
        println!("前端目录：{}", web_dir.display());
    } else {
        println!(
            "提示：未找到前端构建产物（{}）。先在 desktop/ 执行 npm install && npm run build，",
            web_dir.join("index.html").display()
        );
        println!("或用 npm run dev 起开发服务器（已配置 /api 代理到本服务）。");
    }

    let server = Arc::new(server);
    let paths = Arc::new(paths);
    let web_dir = Arc::new(web_dir);

    let mut handles = Vec::new();
    for _ in 0..4 {
        let server = Arc::clone(&server);
        let paths = Arc::clone(&paths);
        let web_dir = Arc::clone(&web_dir);
        handles.push(thread::spawn(move || {
            while let Ok(req) = server.recv() {
                handle(req, &paths, &web_dir);
            }
        }));
    }
    for h in handles {
        let _ = h.join();
    }
    Ok(())
}

fn handle(req: tiny_http::Request, paths: &DashboardPaths, web_dir: &Path) {
    let url = req.url().to_string();
    let (path, query) = match url.split_once('?') {
        Some((p, q)) => (p, q),
        None => (url.as_str(), ""),
    };

    if path.starts_with("/api/") {
        let (status, body) = handle_api(path, query, paths);
        let _ = req.respond(json_response(status, body));
    } else {
        serve_static(req, path, web_dir);
    }
}

/// 处理 API 请求，返回 (状态码, JSON 文本)。
fn handle_api(path: &str, query: &str, paths: &DashboardPaths) -> (u16, String) {
    match path {
        "/api/summary" => match build_summary(paths) {
            Ok(s) => (200, s),
            Err(e) => (500, err_json(&e)),
        },
        "/api/events" => match build_events(paths, query) {
            Ok(s) => (200, s),
            Err(e) => (500, err_json(&e)),
        },
        "/api/servers" => (200, build_servers(paths)),
        "/api/memory" => (200, build_memory(paths)),
        "/api/report" => match build_report(paths, query) {
            Ok(s) => (200, s),
            Err(e) => (500, err_json(&e)),
        },
        _ => (404, json!({ "error": "未知接口" }).to_string()),
    }
}

fn build_summary(paths: &DashboardPaths) -> anyhow::Result<String> {
    let store = SqliteStore::open(&paths.db)?;
    let records = store.query(&EventQuery {
        limit: Some(2000),
        ..Default::default()
    })?;

    let (mut low, mut medium, mut high, mut critical) = (0, 0, 0, 0);
    let mut blocked = 0;
    let mut today = 0;
    let today_prefix = chrono::Utc::now().format("%Y-%m-%d").to_string();
    for r in &records {
        match r.risk_level.as_str() {
            "Low" => low += 1,
            "Medium" => medium += 1,
            "High" => high += 1,
            "Critical" => critical += 1,
            _ => {}
        }
        if r.decision == "Block" {
            blocked += 1;
        }
        if r.created_at.starts_with(&today_prefix) {
            today += 1;
        }
    }
    let active_servers = Config::load(&paths.config)
        .map(|c| c.servers.values().filter(|s| s.enabled).count())
        .unwrap_or(0);

    let recent: Vec<_> = records.iter().take(10).collect();

    Ok(json!({
        "total": records.len(),
        "today": today,
        "blocked": blocked,
        "high_risk": high + critical,
        "active_servers": active_servers,
        "by_level": { "Low": low, "Medium": medium, "High": high, "Critical": critical },
        "recent": recent,
    })
    .to_string())
}

fn build_events(paths: &DashboardPaths, query: &str) -> anyhow::Result<String> {
    let q = parse_query(query);
    let store = SqliteStore::open(&paths.db)?;
    let records = store.query(&EventQuery {
        level: q.get("level").filter(|s| !s.is_empty()).cloned(),
        server: q.get("server").filter(|s| !s.is_empty()).cloned(),
        since: q.get("since").filter(|s| !s.is_empty()).cloned(),
        until: q.get("until").filter(|s| !s.is_empty()).cloned(),
        limit: Some(
            q.get("limit")
                .and_then(|s| s.parse().ok())
                .unwrap_or(100usize),
        ),
    })?;
    Ok(serde_json::to_string(&records)?)
}

fn build_servers(paths: &DashboardPaths) -> String {
    let cfg = Config::load(&paths.config).unwrap_or_default();
    let list: Vec<_> = cfg
        .servers
        .iter()
        .map(|(name, s)| {
            json!({
                "name": name,
                "transport": if s.url.is_some() { "http" } else { "stdio" },
                "upstream": s.url.clone().unwrap_or_else(|| format!("{} {}", s.command, s.args.join(" "))),
                "trust_level": s.trust_level,
                "enabled": s.enabled,
            })
        })
        .collect();
    json!({ "servers": list }).to_string()
}

fn build_memory(paths: &DashboardPaths) -> String {
    let mem = DecisionMemory::load(&paths.decisions);
    let (allow, block) = mem.entries();
    json!({ "allow": allow, "block": block }).to_string()
}

fn build_report(paths: &DashboardPaths, query: &str) -> anyhow::Result<String> {
    let q = parse_query(query);
    let fmt = q
        .get("format")
        .and_then(|f| Format::parse(f))
        .unwrap_or(Format::Markdown);
    let store = SqliteStore::open(&paths.db)?;
    let records = store.query(&EventQuery::default())?;
    let meta = ReportMeta {
        project: "AgentShield".to_string(),
        generated_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
    };
    let report = Report::build(&records, meta);
    let text = report.render(fmt)?;
    // 统一用 JSON 包一层，前端按 text 字段取用（report 的 json 格式本身也是字符串）
    Ok(json!({ "content": text }).to_string())
}

/// 托管前端静态文件，找不到且无扩展名时回落到 index.html（SPA）。
fn serve_static(req: tiny_http::Request, path: &str, web_dir: &Path) {
    let rel = path.trim_start_matches('/');
    let mut file = web_dir.join(rel);
    if rel.is_empty() || !file.exists() {
        // SPA 回落
        if !rel.contains('.') {
            file = web_dir.join("index.html");
        }
    }

    match std::fs::read(&file) {
        Ok(bytes) => {
            let ct = content_type(file.extension().and_then(|e| e.to_str()).unwrap_or(""));
            let header = Header::from_bytes(&b"Content-Type"[..], ct.as_bytes()).unwrap();
            let _ = req.respond(Response::from_data(bytes).with_header(header));
        }
        Err(_) => {
            let _ = req.respond(Response::from_string("Not Found").with_status_code(404));
        }
    }
}

fn json_response(status: u16, body: String) -> Response<std::io::Cursor<Vec<u8>>> {
    let ct = Header::from_bytes(
        &b"Content-Type"[..],
        &b"application/json; charset=utf-8"[..],
    )
    .unwrap();
    // 本地开发时前端可能跑在不同端口，放开 CORS（仅本地使用）
    let cors = Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap();
    Response::from_string(body)
        .with_status_code(status)
        .with_header(ct)
        .with_header(cors)
}

fn content_type(ext: &str) -> &'static str {
    match ext {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn parse_query(query: &str) -> std::collections::HashMap<String, String> {
    query
        .split('&')
        .filter(|kv| !kv.is_empty())
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_string(), urldecode(v)))
        .collect()
}

/// 极简 URL 解码（够用于 level/server/日期等参数）。
fn urldecode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        match b {
            b'+' => out.push(' '),
            b'%' => {
                let h = bytes.next();
                let l = bytes.next();
                if let (Some(h), Some(l)) = (h, l) {
                    if let (Some(h), Some(l)) = ((h as char).to_digit(16), (l as char).to_digit(16))
                    {
                        out.push((h * 16 + l) as u8 as char);
                        continue;
                    }
                }
            }
            _ => out.push(b as char),
        }
    }
    out
}

fn err_json(e: &anyhow::Error) -> String {
    json!({ "error": e.to_string() }).to_string()
}
