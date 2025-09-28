use anyhow::{Context, bail};
use std::path::PathBuf;
use std::process::Stdio;

pub struct TestServerBin(pub(crate) PathBuf);
pub struct P2proxydBin(pub(crate) PathBuf);
pub struct P2proxyCliBin(pub(crate) PathBuf);

pub struct BuiltBinaries {
    pub test_server: TestServerBin,
    pub p2proxyd: P2proxydBin,
    pub p2proxy_cli: P2proxyCliBin,
}

impl BuiltBinaries {
    pub fn build_all() -> anyhow::Result<Self> {
        build_root()?;
        Ok(Self {
            test_server: TestServerBin(built_bin_path("p2proxy-test-server")?),
            p2proxyd: P2proxydBin(built_bin_path("p2proxyd")?),
            p2proxy_cli: P2proxyCliBin(built_bin_path("p2proxy-cli")?),
        })
    }
}

fn build_root() -> anyhow::Result<()> {
    let bin_run = std::process::Command::new("cargo")
        .arg("b")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .context("failed to cargo build")?;
    if !bin_run.status.success() {
        anyhow::bail!("cargo build failed, status={}", bin_run.status);
    }
    Ok(())
}

fn built_bin_path(bin: &str) -> anyhow::Result<PathBuf> {
    let pb = PathBuf::new().join("target").join("debug").join(bin);
    if std::fs::exists(&pb).with_context(|| format!("failed to find binary path for {bin}"))? {
        Ok(pb)
    } else {
        bail!("failed to find binary path for {} at {}", bin, pb.display());
    }
}
