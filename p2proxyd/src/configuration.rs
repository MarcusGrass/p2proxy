use crate::access_log::AccessLogHandle;
use crate::proto::{PortConfig, Routes};
use anyhow::{Context, bail};
use iroh::SecretKey;
use p2proxy_lib::proto::ServerPortMapString;
use rustc_hash::{FxHashMap, FxHashSet};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
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
    pub fn into_cfg(self) -> anyhow::Result<P2ProxydSetup> {
        let toml = P2proxydTomlConfig::from_args(&self)?;
        P2ProxydSetup::from_toml(toml).context("failed to parse p2proxyd config")
    }
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
pub struct P2proxydTomlConfig {
    pub secret_key_path: Option<PathBuf>,
    pub secret_key_hex: Option<String>,
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
    pub allow_any_peer: Option<bool>,
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
            peers: vec![],
            server_ports: vec![ServerPortSetting {
                host_ip: None,
                port: 8080,
                name: "my-http".to_string(),
                allow_any_peer: Some(true),
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

pub struct P2ProxydSetup {
    pub secret_key: SecretKey,
    pub routes: Routes,
    pub access_log_handle: AccessLogHandle,
}

impl P2ProxydSetup {
    pub fn from_toml(p2proxyd_toml_config: P2proxydTomlConfig) -> anyhow::Result<Self> {
        let secret_key = ensure_secret_key(&p2proxyd_toml_config)?;
        let routes = construct_routes(
            p2proxyd_toml_config.default_route,
            p2proxyd_toml_config.server_ports,
            &p2proxyd_toml_config.peers,
        )?;
        let access_log_handle = AccessLogHandle::maybe_spawn(p2proxyd_toml_config.access_log_path);

        Ok(Self {
            secret_key,
            routes,
            access_log_handle,
        })
    }
}

#[allow(clippy::too_many_lines)]
fn construct_routes(
    default_route: Option<String>,
    server_ports: Vec<ServerPortSetting>,
    peers: &[PeerPermission],
) -> anyhow::Result<Routes> {
    let mut paths_unique = FxHashSet::default();
    let default_route = if let Some(dr_path) = default_route {
        let spm = ServerPortMapString::try_new(dr_path)?;
        paths_unique.insert(spm.clone());
        Some(spm)
    } else {
        None
    };
    let mut default_route_hit = None;
    let mut route_config = FxHashMap::default();
    for p in server_ports {
        let server_port_name = ServerPortMapString::try_new(p.name.clone()).with_context(|| {
            format!(
                "configuration error: server port map string={} is invalid",
                p.name
            )
        })?;
        let is_default_route = if paths_unique.insert(server_port_name.clone()) {
            false
        } else if let Some(dr) = &default_route {
            if default_route_hit.is_some() {
                bail!("configuration error: server port name duplication on default route {dr}");
            }
            true
        } else {
            bail!("configuration error: server port name {server_port_name} is not unique");
        };
        let addr = SocketAddr::new(
            p.host_ip.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
            p.port,
        );
        if p.allow_any_peer == Some(true) {
            let config = PortConfig::new(None, addr);
            if is_default_route {
                default_route_hit = Some(config.clone());
            }
            route_config.insert(server_port_name, config);
            continue;
        }
        let mut explicit_allow_map = FxHashSet::default();
        for peer in peers {
            if peer.allow_any_port {
                explicit_allow_map.insert(peer.node_id);
                continue;
            }
            // Unnecessary double-loop, if someone complains about start-up times and
            // this is the cause, I'll eat my hat. And then maybe change this to be less wasteful.
            if let Some(named_ports) = &peer.allow_named_ports {
                let mut peer_port_set = FxHashSet::default();
                for peer_port in named_ports {
                    // Just validation
                    if !peer_port_set.insert(peer_port) {
                        anyhow::bail!(
                            "configuration error, peer={} specified a duplicate named port={}",
                            peer.node_id,
                            peer_port
                        );
                    }
                    let spm =
                        ServerPortMapString::try_new(peer_port.clone()).with_context(|| {
                            format!(
                                "configuration error, peer={} specified an invalid named port={}",
                                peer.node_id, peer_port
                            )
                        })?;
                    if server_port_name == spm {
                        explicit_allow_map.insert(peer.node_id);
                    }
                }
            }
        }
        if explicit_allow_map.is_empty() {
            anyhow::bail!(
                "configuration error, server port {} has no explicit allow list, and does not allow any (cannot be connected to)",
                server_port_name
            );
        }

        let config = PortConfig::new(Some(explicit_allow_map), addr);
        if is_default_route {
            default_route_hit = Some(config.clone());
        }
        route_config.insert(server_port_name, config);
    }

    let default_route_spec = match (default_route, default_route_hit) {
        (None, None) => None,
        (Some(wants), None) => {
            anyhow::bail!(
                "configuration error: default route '{wants}' specified, but no server ports expose it"
            );
        }
        (None, Some(hit)) => {
            anyhow::bail!(
                "configuration error: parse error, no default route specified, but a path for it at '{}', this is a bug",
                hit.socket_addr
            );
        }
        (Some(_), Some(hit)) => Some(hit),
    };

    Ok(Routes::new(default_route_spec, route_config))
}

fn ensure_secret_key(config: &P2proxydTomlConfig) -> anyhow::Result<SecretKey> {
    match (&config.secret_key_path, &config.secret_key_hex) {
        (Some(p), Some(hex)) => {
            tracing::warn!("supplied both a secret key from file and as an argument");
            // Reading both to ensure there isn't a mismatch which the user should fix
            let secret_from_hex =
                SecretKey::from_str(hex).context("failed to parse secret key hex")?;
            let secret_from_file = read_secret_key_from_file(p)?;
            if secret_from_hex.to_bytes() != secret_from_file.to_bytes() {
                return Err(anyhow::anyhow!(
                    "supplied both a secret key from file and as an argument, and they don't match"
                ));
            }
            Ok(secret_from_file)
        }
        (Some(p), None) => {
            let secret_from_file = read_secret_key_from_file(p)?;
            Ok(secret_from_file)
        }
        (None, Some(hex)) => {
            let secret_from_hex =
                SecretKey::from_str(hex).context("failed to parse secret key hex")?;
            Ok(secret_from_hex)
        }
        (None, None) => {
            bail!("either a secret key from file or hex as an argument must be supplied");
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
