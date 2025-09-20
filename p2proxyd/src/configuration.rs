use anyhow::{Context, bail};
use iroh::{NodeId, SecretKey};
use p2proxy_lib::proto::ServerPortMapString;
use rustc_hash::FxHashSet;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Run the p2proxy daemon
#[derive(clap::Parser, Debug)]
pub struct P2proxydCliArgs {
    /// Path to the configuration file
    #[clap(short, long)]
    pub cfg_path: PathBuf,
}

impl P2proxydCliArgs {
    pub fn into_cfg(self) -> anyhow::Result<P2proxydConfig> {
        let toml = P2proxydTomlConfig::from_args(&self)?;
        P2proxydConfig::from_toml(toml).context("failed to parse p2proxyd config")
    }
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
pub struct P2proxydTomlConfig {
    pub secret_key_path: Option<PathBuf>,
    pub secret_key_hex: Option<String>,
    pub allow_any_peer: bool,
    pub peers: Vec<PeerPermission>,
    pub server_ports: Vec<ServerPortSetting>,
    pub access_log_path: Option<PathBuf>,
    pub default_route: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ServerPortSetting {
    pub host_ip: Option<IpAddr>,
    pub port: u16,
    pub name: String,
}

#[derive(Debug, Eq, PartialEq, Hash, serde::Deserialize, serde::Serialize)]
pub struct PeerPermission {
    pub node_id: iroh::NodeId,
    pub allow_any_port: bool,
    pub allow_named_ports: Option<Vec<String>>,
}

impl P2proxydTomlConfig {
    pub fn from_args(p2proxyd_cli_args: &P2proxydCliArgs) -> anyhow::Result<Self> {
        let content = std::fs::read(&p2proxyd_cli_args.cfg_path).with_context(|| {
            format!(
                "failed to read p2proxyd config file: {}",
                p2proxyd_cli_args.cfg_path.display()
            )
        })?;

        Self::parse_toml(&content)
    }

    pub fn generate_template_to_toml() -> anyhow::Result<String> {
        let key = iroh::SecretKey::generate(&mut rand::rngs::OsRng);
        let secret_key_hex = hex::encode(key.to_bytes());
        tracing::info!("generated secret key with node id: {}", key.public());
        let slf = Self {
            secret_key_path: None,
            secret_key_hex: Some(secret_key_hex),
            allow_any_peer: true,
            peers: vec![],
            server_ports: vec![ServerPortSetting {
                host_ip: None,
                port: 8080,
                name: "my-http".to_string(),
            }],
            access_log_path: None,
            default_route: Some("my-http".to_string()),
        };
        toml::to_string(&slf).context("failed to serialize p2proxyd config")
    }

    #[inline]
    pub fn parse_toml(toml: &[u8]) -> anyhow::Result<Self> {
        toml::from_slice(toml).context("failed to deserialize p2proxyd config")
    }
}

pub struct P2proxydConfig {
    pub secret_key: SecretKey,
    pub peers: Option<FxHashSet<NodeId>>,
    pub server_port: Vec<ServerPortSetting>,
    pub access_log: Option<PathBuf>,
    pub default_route: Option<ServerPortMapString>,
}

impl P2proxydConfig {
    pub fn from_toml(mut p2proxyd_toml_config: P2proxydTomlConfig) -> anyhow::Result<Self> {
        let peers = if !p2proxyd_toml_config.allow_any_peer && p2proxyd_toml_config.peers.is_empty()
        {
            bail!("at least one peer must be specified if allow_any_peer is false");
        } else if p2proxyd_toml_config.allow_any_peer {
            None
        } else {
            Some(
                p2proxyd_toml_config
                    .peers
                    .into_iter()
                    .map(|p| p.node_id)
                    .collect(),
            )
        };
        if p2proxyd_toml_config.server_ports.is_empty() {
            bail!("at least one server port must be specified");
        }
        let default_route = p2proxyd_toml_config
            .default_route
            .map(ServerPortMapString::try_new)
            .transpose()?;
        for p in &mut p2proxyd_toml_config.server_ports {
            if p.name.len() > 16 {
                if p.name.is_char_boundary(16) {
                    bail!(
                        "server port name too long {}, max 16 chars, and was unable to truncate it",
                        p.name
                    );
                }
                p.name.truncate(16);
                tracing::warn!("server port name truncated to 16 chars");
            }
        }
        match (
            p2proxyd_toml_config.secret_key_path,
            p2proxyd_toml_config.secret_key_hex,
        ) {
            (Some(p), Some(hex)) => {
                tracing::warn!("supplied both a secret key from file and as an argument");
                // Reading both to ensure there isn't a mismatch which the user should fix
                let secret_from_hex =
                    SecretKey::from_str(&hex).context("failed to parse secret key hex")?;
                let secret_from_file = read_secret_key_from_file(&p)?;
                if secret_from_hex.to_bytes() != secret_from_file.to_bytes() {
                    return Err(anyhow::anyhow!(
                        "supplied both a secret key from file and as an argument, and they don't match"
                    ));
                }
                Ok(Self {
                    secret_key: secret_from_file,
                    peers,
                    server_port: p2proxyd_toml_config.server_ports,
                    access_log: p2proxyd_toml_config.access_log_path,
                    default_route,
                })
            }
            (Some(p), None) => {
                let secret_from_file = read_secret_key_from_file(&p)?;
                Ok(Self {
                    secret_key: secret_from_file,
                    peers,
                    server_port: p2proxyd_toml_config.server_ports,
                    access_log: p2proxyd_toml_config.access_log_path,
                    default_route,
                })
            }
            (None, Some(hex)) => {
                let secret_from_hex =
                    SecretKey::from_str(&hex).context("failed to parse secret key hex")?;
                Ok(Self {
                    secret_key: secret_from_hex,
                    peers,
                    server_port: p2proxyd_toml_config.server_ports,
                    access_log: p2proxyd_toml_config.access_log_path,
                    default_route,
                })
            }
            (None, None) => {
                bail!("either a secret key from file or hex as an argument must be supplied");
            }
        }
    }
}

fn read_secret_key_from_file(p: &Path) -> anyhow::Result<SecretKey> {
    let secret_from_file = std::fs::read(p)
        .with_context(|| format!("failed to read supplied secret key file: {}", p.display()))?
        .try_into()
        .map_err(|_e| {
            anyhow::anyhow!(
                "failed to parse secret key file: {}, invalid length",
                p.display()
            )
        })?;
    Ok(SecretKey::from_bytes(&secret_from_file))
}
