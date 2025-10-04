#![cfg_attr(target_family = "windows", windows_subsystem = "windows")]
mod observability;

use crate::observability::setup_observability;
use anyhow::Context;
use clap::Parser;
use iroh::Endpoint;
use p2proxy_client::killswitch::ProxyKillSwitch;
use p2proxy_lib::display_chain;
use p2proxy_lib::proto::ServerPortMapString;
use std::path::PathBuf;
use std::process::ExitCode;
use tokio::runtime::LocalRuntime;

#[derive(Debug, clap::Parser)]
pub struct Args {
    #[clap(subcommand)]
    command: Subcommand,
}

#[derive(Debug, clap::Parser)]
#[command(version, about, long_about = None)]
pub enum Subcommand {
    /// Generate a new secret key
    GenerateKey {
        /// The file where the secret key should be written
        #[clap(short, long)]
        dest: PathBuf,
    },
    /// Serve the proxy on a local port
    Serve {
        /// The path to a file containing this node's secret key.
        /// Either this, or `key_hex` needs to be set.
        #[clap(long, env)]
        key_path: Option<PathBuf>,
        /// This node's secret key, hex encoded.
        /// Either this, or `key_path` needs to be set.
        #[clap(long, env)]
        key_hex: Option<String>,
        /// The hex-encoded public key (node id) of the peer to connect to.
        #[clap(long, env)]
        peer: iroh::NodeId,
        /// The local port to serve the proxy on.
        #[clap(long, env)]
        local_port: u16,
        /// The optional remote port routing name.
        #[clap(long, env)]
        named_port: Option<String>,
    },
}

fn main() -> ExitCode {
    let args = Args::parse();
    let runtime = LocalRuntime::new().expect("failed to create p2proxyd runtime");
    match runtime.block_on(run_app(args)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("p2proxy-cli failed -> {}", display_chain(&*e));
            ExitCode::FAILURE
        }
    }
}

async fn run_app(args: Args) -> anyhow::Result<()> {
    setup_observability();
    match args.command {
        Subcommand::GenerateKey { dest } => {
            let key = iroh::SecretKey::generate(&mut rand::rngs::OsRng);
            std::fs::write(&dest, key.to_bytes())
                .with_context(|| format!("failed to write key to {}", dest.display()))?;
            println!(
                "key {} written to {}, node_id={}",
                hex::encode(key.to_bytes()),
                dest.display(),
                key.public()
            );
            Ok(())
        }
        Subcommand::Serve {
            key_hex,
            key_path,
            peer,
            local_port,
            named_port: remote_port_name,
        } => {
            let rmp = if let Some(p) = remote_port_name {
                Some(ServerPortMapString::try_new(p).context("failed to create server port map")?)
            } else {
                None
            };
            let key = if let Some(key_hex) = key_hex {
                let key_material = hex::decode(&key_hex)
                    .context("failed to decode supplied secret key hex")?
                    .try_into()
                    .map_err(|_e| anyhow::anyhow!("supplied secret key hex is incorrect length"))?;
                iroh::SecretKey::from_bytes(&key_material)
            } else if let Some(key_path) = key_path {
                let key_material = std::fs::read(&key_path)
                    .with_context(|| format!("failed to read key from {}", key_path.display()))?
                    .try_into()
                    .map_err(|_e| {
                        anyhow::anyhow!(
                            "suplied secret key file contains bytes of an incorrect length"
                        )
                    })?;
                iroh::SecretKey::from_bytes(&key_material)
            } else {
                anyhow::bail!("key-hex or key-path needs to be supplied");
            };
            let ep = Endpoint::builder()
                .secret_key(key)
                .discovery_n0()
                .bind()
                .await
                .context("failed to bind endpoint")?;
            let (_ks, listen) = ProxyKillSwitch::new_pair();
            let mut receiver = p2proxy_client::spawn_serve_with_updates_killswitched(
                ep, peer, local_port, rmp, listen,
            );
            while let Some(update) = receiver.recv().await {
                let up = update?;
                tracing::info!("received update: {up:?}");
            }
            Ok(())
        }
    }
}
