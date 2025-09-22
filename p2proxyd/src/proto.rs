mod connection;

use crate::access_log::AccessLogHandle;
use crate::proto::connection::spawn_client_connection;
use iroh::NodeId;
use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};
use p2proxy_lib::display_chain;
use p2proxy_lib::proto::ServerPortMapString;
use rustc_hash::{FxHashMap, FxHashSet};
use std::net::SocketAddr;

#[derive(Debug)]
pub(crate) struct Routes {
    default: Option<PortConfig>,
    inner: FxHashMap<ServerPortMapString, PortConfig>,
}

#[derive(Debug, Clone)]
pub struct PortConfig {
    // An empty here means allow any
    pub allowed_peers: Option<FxHashSet<NodeId>>,
    pub socket_addr: SocketAddr,
}

impl PortConfig {
    pub fn new(allowed_peers: Option<FxHashSet<NodeId>>, socket_addr: SocketAddr) -> Self {
        Self {
            allowed_peers,
            socket_addr,
        }
    }

    #[inline]
    fn is_allowed(&self, nid: &NodeId) -> bool {
        self.allowed_peers
            .as_ref()
            .is_none_or(|allowed_peers| allowed_peers.contains(nid))
    }
}

pub enum SocketAddrGetResult {
    Allowed(SocketAddr),
    NotAllowed,
    NotPresent,
}

impl Routes {
    pub(crate) fn new(
        default: Option<PortConfig>,
        inner: FxHashMap<ServerPortMapString, PortConfig>,
    ) -> Self {
        Self { default, inner }
    }

    #[inline]
    fn get(&self, node: &NodeId, port: &str) -> SocketAddrGetResult {
        let Some(port_cfg) = self.inner.get(port) else {
            return SocketAddrGetResult::NotPresent;
        };
        if !port_cfg.is_allowed(node) {
            return SocketAddrGetResult::NotAllowed;
        }
        SocketAddrGetResult::Allowed(port_cfg.socket_addr)
    }

    #[inline]
    pub fn default_route(&self, node_id: &NodeId) -> SocketAddrGetResult {
        match &self.default {
            None => SocketAddrGetResult::NotPresent,
            Some(cfg) => {
                if cfg.is_allowed(node_id) {
                    SocketAddrGetResult::Allowed(cfg.socket_addr)
                } else {
                    SocketAddrGetResult::NotAllowed
                }
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct DownstreamConnectionInheritedState {
    pub(super) routes: Routes,
    pub(super) access_log_handle: AccessLogHandle,
}

#[derive(Debug)]
pub struct P2ProxyProto {
    inherited: &'static DownstreamConnectionInheritedState,
}
impl P2ProxyProto {
    pub fn new(routes: Routes, access_log_handle: AccessLogHandle) -> Self {
        let inherited = DownstreamConnectionInheritedState {
            routes,
            access_log_handle,
        };
        // Having an Arc for this is just unnecessary since this memory is never released.
        // Just leak it.
        let inherited = Box::leak(Box::new(inherited));
        Self { inherited }
    }
}

impl ProtocolHandler for P2ProxyProto {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let addr = connection.remote_address();
        let nid = match connection.remote_node_id() {
            Ok(nid) => nid,
            Err(e) => {
                if let Err(e) = self
                    .inherited
                    .access_log_handle
                    .log_rejected_missing_node_id(addr)
                {
                    tracing::error!("failed to log rejected connection: {}", display_chain(&*e));
                }
                tracing::warn!("unknown remote node connected: {}", display_chain(&e));
                connection.close(
                    p2proxy_lib::proto::GENERIC_QUIC_ERROR_CODE,
                    b"missing node id",
                );
                return Err(AcceptError::NotAllowed {});
            }
        };
        if let Err(e) = self.inherited.access_log_handle.log_accepted(addr, nid) {
            tracing::error!("failed to log accepted connection: {}", display_chain(&*e));
        }
        spawn_client_connection(nid, addr, connection, self.inherited);
        tracing::debug!("accepted connection from {nid}");
        Ok(())
    }
}
