use anyhow::{Context, bail};
use iroh::{NodeId, SecretKey};
use p2proxy_lib::proto::ServerPortMapString;
use rustc_hash::FxHashSet;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Run the p2proxy daemon
#[derive(clap::Parser, Debug)]
pub struct P2proxydCliArgs {
    /// Path to a configuration file, if supplied together with other arguments, the arguments
    /// will override fields from the loaded configuration.
    #[clap(long)]
    pub cfg_path: Option<PathBuf>,
    /// Path to the secret key for this node.
    /// If supplied together with `secret_key_hex` one will be picked un-deterministically.
    #[clap(long)]
    pub secret_key_path: Option<PathBuf>,
    /// The hexadecimal representation of the 32-byte secret key for this node.
    /// If supplied together with `secret_key_path` one will be picked un-deterministically.
    #[clap(long)]
    pub secret_key_hex: Option<String>,
    /// Allow any peer to connect, defaults to false if omitted.
    /// If `true` anyone that knows this node's node id can connect to it.
    #[clap(long)]
    pub allow_any_peer: Option<bool>,
    /// Downstream TCP server ports. Each entry needs a corresponding name in
    /// `server_port_names`
    /// Ex: `3100,7777,61203`
    #[clap(long)]
    pub server_ports: Vec<u16>,
    /// Mapping of names to server ports. Each entry needs a corresponding port in
    /// `server_ports`. Only the first 16 bytes will be used (16 chars if ASCII), the
    /// rest will be discarded silently.
    /// Ex: `node,secret-service,high`
    #[clap(long)]
    pub server_port_names: Vec<String>,
    /// A default port route, allowing clients to not have to supply (or know) what port mappings
    /// exist. Carries the same name constraints as `server_port_names`.
    /// Overwrites configuration if present, but not if absent.
    pub default_route: Option<String>,
    /// The node id of peers that are allowed to connect to this node.
    /// Useless if `allow_any_peer == true`, mandatory if `allow_any_peer == false`.
    /// It's assumed that nodes entered here are allowed on any port.
    /// If more granular control is desired, use the `toml` configuration
    #[clap(long)]
    pub peer_node_id: Vec<iroh::NodeId>,
    #[clap(long)]
    pub peer_node_any_port: Vec<bool>,
    /// If supplied, the proxy will keep an access log at the path specified.
    /// The file will be appended to forever, employ log rotation if that behaviour is unwanted.
    /// Ex: `/var/log/p2proxyd.log`
    #[clap(long)]
    pub access_log_path: Option<PathBuf>,
}

impl P2proxydCliArgs {
    pub fn into_cfg(self) -> anyhow::Result<P2proxydConfig> {
        let toml = P2proxydTomlConfig::from_args(self)?;
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
    pub fn from_args(p2proxyd_cli_args: P2proxydCliArgs) -> anyhow::Result<Self> {
        let mut base = if let Some(cfg_path) = p2proxyd_cli_args.cfg_path {
            let content = std::fs::read(&cfg_path).with_context(|| {
                format!(
                    "failed to read p2proxyd config file: {}",
                    cfg_path.display()
                )
            })?;

            Self::parse_toml(&content)?
        } else {
            Self::default()
        };
        if p2proxyd_cli_args.secret_key_path.is_some() {
            base.secret_key_path = p2proxyd_cli_args.secret_key_path;
        }
        if p2proxyd_cli_args.secret_key_hex.is_some() {
            base.secret_key_hex = p2proxyd_cli_args.secret_key_hex;
        }
        if let Some(allow_any_peer) = p2proxyd_cli_args.allow_any_peer {
            base.allow_any_peer = allow_any_peer;
        }
        if p2proxyd_cli_args.server_port_names.len() != p2proxyd_cli_args.server_ports.len() {
            bail!(
                "server port names and ports must be of the same length, they correspond to each other"
            );
        }
        if let Some(default_route) = p2proxyd_cli_args.default_route {
            base.default_route = Some(default_route);
        }
        let cli_ports: Vec<_> = p2proxyd_cli_args
            .server_ports
            .into_iter()
            .zip(p2proxyd_cli_args.server_port_names)
            .map(|(port, name)| ServerPortSetting { port, name })
            .collect();
        if !cli_ports.is_empty() {
            base.server_ports = cli_ports;
        }
        if let Some(access_log_path) = p2proxyd_cli_args.access_log_path {
            base.access_log_path = Some(access_log_path);
        }
        if !p2proxyd_cli_args.peer_node_id.is_empty() {
            base.peers = p2proxyd_cli_args
                .peer_node_id
                .into_iter()
                .map(|nid| PeerPermission {
                    node_id: nid,
                    allow_any_port: true,
                    allow_named_ports: None,
                })
                .collect();
        }
        Ok(base)
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
