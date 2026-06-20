//! AgentShield 桌面外壳。
//!
//! 不重写任何业务逻辑：进程内启动 `agentshield-dashboard` 的本地 HTTP 服务，
//! 再用一个原生窗口加载它（tauri.conf.json 里窗口 url = http://127.0.0.1:8787）。
//! 前端与单纯浏览器访问完全一致，数据仍来自本机 `.agentshield/`。
//!
//! 监控的项目目录默认取当前工作目录，可用环境变量 `AGENTSHIELD_HOME` 覆盖。

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use agentshield_dashboard::{run_dashboard, DashboardPaths};

const ADDR: &str = "127.0.0.1:8787";

fn main() {
    // 监控的项目目录：环境变量优先，否则当前工作目录
    let home = std::env::var("AGENTSHIELD_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
    let paths = DashboardPaths {
        db: home.join(".agentshield/audit.db"),
        config: home.join(".agentshield/config.yaml"),
        decisions: home.join(".agentshield/decisions.json"),
    };
    // 前端构建产物随源码定位到 desktop/dist
    let web_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dist");

    // 后台启动仪表盘服务，窗口随后加载它
    thread::spawn(move || {
        if let Err(e) = run_dashboard(ADDR, web_dir, paths) {
            eprintln!("[AgentShield] 仪表盘服务退出：{e}");
        }
    });
    // 给服务一点启动时间，避免窗口首帧加载到空页面
    thread::sleep(Duration::from_millis(600));

    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("启动 Tauri 失败");
}
