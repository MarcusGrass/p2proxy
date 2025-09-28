use crate::build::{P2proxyCliBin, P2proxydBin, TestServerBin};
use anyhow::Context;
use iroh::{NodeId, SecretKey};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Child;

struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        self.0.start_kill().unwrap();
    }
}

fn spawn_bin(binary_path: &Path, args: &[&str]) -> anyhow::Result<KillOnDrop> {
    Ok(KillOnDrop(
        tokio::process::Command::new(binary_path)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to spawn binary")?,
    ))
}

pub struct RunningP2Proxyd {
    child: KillOnDrop,
}

impl RunningP2Proxyd {
    pub fn run_with_cfg(binary: &P2proxydBin, cfg_path: &str) -> anyhow::Result<Self> {
        let process = spawn_bin(&binary.0, &["run", "--cfg-path", cfg_path])?;
        Ok(Self { child: process })
    }

    pub async fn dump_output(&mut self) {
        println!("Output dump from running-p2proxyd");
        dump_output(&mut self.child).await;
    }
}

pub struct RunningTestServer {
    child: KillOnDrop,
}

impl RunningTestServer {
    pub fn run_on_port(binary: &TestServerBin, port: u16) -> anyhow::Result<Self> {
        let process = spawn_bin(&binary.0, &[&port.to_string()])?;
        Ok(Self { child: process })
    }

    pub async fn dump_output(&mut self) {
        println!("Output dump from running-test-server");
        dump_output(&mut self.child).await;
    }
}

pub struct RunningCli {
    pub port: u16,
    child: KillOnDrop,
}

impl RunningCli {
    pub fn run_with_key(
        binary: &P2proxyCliBin,
        secret_key: &SecretKey,
        local_port: u16,
        peer: &NodeId,
        named_port: Option<&str>,
    ) -> anyhow::Result<Self> {
        let hex = hex::encode(secret_key.to_bytes());
        let local_port_s = local_port.to_string();
        let peer = peer.to_string();
        let mut args = vec![
            "serve",
            "--key-hex",
            hex.as_str(),
            "--local-port",
            local_port_s.as_str(),
            "--peer",
            peer.as_str(),
        ];
        if let Some(named_port) = named_port {
            args.push("--named-port");
            args.push(named_port);
        }
        let running_cli = spawn_bin(&binary.0, &args)?;
        Ok(Self {
            child: running_cli,
            port: local_port,
        })
    }

    pub async fn dump_output(&mut self) {
        println!("Output dump from running-cli");
        dump_output(&mut self.child).await;
    }
}

async fn dump_output(child: &mut KillOnDrop) {
    let mut buf = [0u8; 4096];
    if let Some(stdout) = child.0.stdout.as_mut() {
        let read_bytes = timed_read(stdout, &mut buf).await;
        println!(
            "\tstdout: '{}'",
            String::from_utf8_lossy(&buf[..read_bytes])
        );
    }
    if let Some(stderr) = child.0.stderr.as_mut() {
        let read_bytes = timed_read(stderr, &mut buf).await;
        println!(
            "\tstderr: '{}'",
            String::from_utf8_lossy(&buf[..read_bytes])
        );
    }
}

async fn timed_read<R: AsyncRead + Unpin>(r: &mut R, buf: &mut [u8]) -> usize {
    tokio::time::timeout(Duration::from_millis(200), async {
        r.read(buf).await.unwrap_or_default()
    })
    .await
    .unwrap_or_default()
}
