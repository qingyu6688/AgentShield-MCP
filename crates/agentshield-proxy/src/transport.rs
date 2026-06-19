//! stdio 传输：把真实 MCP server 作为子进程拉起，对接它的 stdin/stdout。

use std::collections::BTreeMap;
use std::io::{self, BufReader};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// 与上游真实 MCP server 的连接。
pub struct Upstream {
    pub child: Child,
    pub stdin: ChildStdin,
    pub stdout: BufReader<ChildStdout>,
}

/// 拉起上游 server 子进程。
///
/// 上游的 stderr 直接继承到当前进程，便于排查上游自身的报错；
/// stdin / stdout 用管道接管，作为 MCP 通道。
pub fn spawn_upstream(
    command: &str,
    args: &[String],
    env: &BTreeMap<String, String>,
) -> io::Result<Upstream> {
    let mut cmd = Command::new(command);
    cmd.args(args)
        .envs(env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    let mut child = cmd.spawn().map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("启动上游 MCP server `{command}` 失败：{e}"),
        )
    })?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("无法获取上游 stdin"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("无法获取上游 stdout"))?;

    Ok(Upstream {
        child,
        stdin,
        stdout: BufReader::new(stdout),
    })
}
