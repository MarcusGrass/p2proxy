mod access_log;
mod configuration;
mod observability;
mod proto;
mod proxy;

use crate::configuration::{P2proxydCliArgs, P2proxydTomlConfig};
use crate::observability::setup_observability;
use anyhow::Context;
use clap::Parser;
use p2proxy_lib::display_chain;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use tokio::runtime::LocalRuntime;

#[derive(clap::Parser, Debug)]
struct Args {
    #[clap(subcommand)]
    subcommand: Subcommand,
}

/// A cli to run, or bootstrap configuration, for a p2proxy daemon
#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
enum Subcommand {
    Run {
        #[clap(flatten)]
        args: P2proxydCliArgs,
    },
    /// Generate a template configuration
    GenerateTemplateConfiguration {
        /// The path to write the template configuration to
        #[clap(long)]
        dest: PathBuf,
    },
}

fn main() -> ExitCode {
    let args = Args::parse();
    let runtime = LocalRuntime::new().expect("failed to create p2proxyd runtime");
    match runtime.block_on(run_app(args)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("p2proxyd failed -> {}", display_chain(&*e));
            ExitCode::FAILURE
        }
    }
}

async fn run_app(args: Args) -> anyhow::Result<()> {
    setup_observability();
    match args.subcommand {
        Subcommand::Run { args } => {
            let cfg = args.into_cfg()?;
            proxy::run_proxy(cfg).await
        }
        Subcommand::GenerateTemplateConfiguration { dest } => generate_template(&dest),
    }
}

fn generate_template(dest: &Path) -> anyhow::Result<()> {
    let content = P2proxydTomlConfig::generate_template_to_toml()?;
    std::fs::write(dest, content)
        .with_context(|| format!("failed to write secret key to {}", dest.display()))?;
    tracing::info!("secret key written to {}", dest.display());
    Ok(())
}
