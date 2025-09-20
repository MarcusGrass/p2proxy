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
    default: Option<SocketAddr>,
    inner: FxHashMap<ServerPortMapString, SocketAddr>,
}

impl Routes {
    pub(crate) fn new(
        default: Option<SocketAddr>,
        inner: FxHashMap<ServerPortMapString, SocketAddr>,
    ) -> Self {
        Self { default, inner }
    }

    #[inline]
    fn get(&self, port: &str) -> Option<SocketAddr> {
        self.inner.get(port).copied()
    }

    #[inline]
    pub fn default_route(&self) -> Option<SocketAddr> {
        self.default
    }
}

#[derive(Debug)]
pub(super) struct DownstreamConnectionInheritedState {
    pub(super) routes: Routes,
    pub(super) access_log_handle: AccessLogHandle,
}

#[derive(Debug)]
pub struct P2ProxyProto {
    allowed_peers: Option<FxHashSet<NodeId>>,
    inherited: &'static DownstreamConnectionInheritedState,
}
impl P2ProxyProto {
    pub fn new(
        allowed_peers: Option<FxHashSet<NodeId>>,
        routes: Routes,
        access_log_handle: AccessLogHandle,
    ) -> Self {
        let inherited = DownstreamConnectionInheritedState {
            routes,
            access_log_handle,
        };
        // Having an Arc for this is just unnecessary since this memory is never released.
        // Just leak it.
        let inherited = Box::leak(Box::new(inherited));
        Self {
            allowed_peers,
            inherited,
        }
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
        if let Some(allowed_peers) = &self.allowed_peers
            && !allowed_peers.contains(&nid)
        {
            if let Err(e) = self
                .inherited
                .access_log_handle
                .log_rejected_not_allowed(addr, nid)
            {
                tracing::error!("failed to log rejected connection: {}", display_chain(&*e));
            }
            tracing::info!("rejecting peer not on allowed list: {nid}");
            connection.close(p2proxy_lib::proto::FORBIDDEN_QUIC_ERROR_CODE, b"forbidden");
            return Err(AcceptError::NotAllowed {});
        }
        if let Err(e) = self.inherited.access_log_handle.log_accepted(addr, nid) {
            tracing::error!("failed to log accepted connection: {}", display_chain(&*e));
        }
        spawn_client_connection(nid, addr, connection, self.inherited);
        tracing::debug!("accepted connection from {nid}");
        Ok(())
    }
}
