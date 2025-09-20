use crate::proto::DownstreamConnectionInheritedState;
use anyhow::{Context, bail};
use iroh::NodeId;
use iroh::endpoint::{Connection, ConnectionError, RecvStream, SendStream};
use p2proxy_lib::display_chain;
use p2proxy_lib::proto::HEADER_LENGTH;
use p2proxy_lib::proxy_copy_buf::{BufCopyError, BufferedCopy};
use std::net::SocketAddr;

pub fn spawn_client_connection(
    peer: NodeId,
    remote_addr: SocketAddr,
    upstream_connection: Connection,
    downstream_connection_inherited_state: &'static DownstreamConnectionInheritedState,
) {
    tokio::task::spawn_local(async move {
        if let Err(e) = run_client_connection(
            peer,
            remote_addr,
            upstream_connection,
            downstream_connection_inherited_state,
        )
        .await
        {
            tracing::error!("client connection error: {}", display_chain(&*e));
        }
    });
}

async fn run_client_connection(
    peer: NodeId,
    remote_addr: SocketAddr,
    upstream_connection: Connection,
    downstream_connection_inherited_state: &'static DownstreamConnectionInheritedState,
) -> anyhow::Result<()> {
    loop {
        // For each unique incoming connection, spawn a new TCP connection downstream
        let res = upstream_connection.accept_bi().await;
        let (upstream_write, upstream_read) = match res {
            Ok(o) => o,
            Err(e) => match map_con_err(&e) {
                Ok(s) => {
                    tracing::debug!("{s}");
                    return Ok(());
                }
                Err(e) => {
                    bail!(
                        "failed to accept upstream connection: {}",
                        display_chain(&*e)
                    );
                }
            },
        };
        tokio::task::spawn_local(async move {
            if let Err(e) = run_proxied_tcp(
                peer,
                remote_addr,
                upstream_write,
                upstream_read,
                downstream_connection_inherited_state,
            )
            .await
            {
                tracing::error!("proxied tcp connection error: {}", display_chain(&*e));
            }
        });
    }
}

fn map_con_err(e: &ConnectionError) -> anyhow::Result<&'static str> {
    match e {
        ConnectionError::VersionMismatch
        | ConnectionError::TransportError(_)
        | ConnectionError::Reset
        | ConnectionError::TimedOut
        | ConnectionError::LocallyClosed
        | ConnectionError::CidsExhausted => {
            bail!("connection error: {}", display_chain(&e));
        }
        ConnectionError::ConnectionClosed(_) | ConnectionError::ApplicationClosed(_) => {
            Ok("connection closed")
        }
    }
}

async fn run_proxied_tcp(
    peer: NodeId,
    remote_addr: SocketAddr,
    mut upstream_write: SendStream,
    mut upstream_read: RecvStream,
    downstream_connection_inherited_state: &'static DownstreamConnectionInheritedState,
) -> anyhow::Result<()> {
    let mut buf = [0u8; HEADER_LENGTH];
    upstream_read
        .read_exact(&mut buf)
        .await
        .context("failed to write hello to upstream")?;
    let downstream_addr = match &buf {
        p2proxy_lib::proto::PING => {
            tracing::debug!("received ping from upstream");
            upstream_write.write_all(b"PONG").await?;
            let _ = upstream_write.finish();
            let _ = upstream_read.stop(p2proxy_lib::proto::QUIC_OK_ERROR_CODE);
            return Ok(());
        }
        p2proxy_lib::proto::DEFAULT_ROUTE => {
            if let Some(default_route) =
                downstream_connection_inherited_state.routes.default_route()
            {
                default_route
            } else {
                downstream_connection_inherited_state
                    .access_log_handle
                    .log_rejected_default_not_present(remote_addr, peer)?;
                let _ = upstream_write.reset(p2proxy_lib::proto::FORBIDDEN_QUIC_ERROR_CODE);
                let _ = upstream_read.stop(p2proxy_lib::proto::FORBIDDEN_QUIC_ERROR_CODE);
                bail!("no default route configured");
            }
        }
        any => {
            let utf8_port_map =
                core::str::from_utf8(&buf).context("invalid utf8 port name mapping")?;
            if let Some(target_port) = downstream_connection_inherited_state
                .routes
                .get(utf8_port_map)
            {
                tracing::debug!("accepted request from upstream");
                target_port
            } else {
                downstream_connection_inherited_state
                    .access_log_handle
                    .log_rejected_bad_port_mapping(remote_addr, peer, utf8_port_map.to_string())?;
                let _ = upstream_write.reset(p2proxy_lib::proto::FORBIDDEN_QUIC_ERROR_CODE);
                let _ = upstream_read.stop(p2proxy_lib::proto::FORBIDDEN_QUIC_ERROR_CODE);
                anyhow::bail!(
                    "unknown port name mapping: {}",
                    String::from_utf8_lossy(any)
                );
            }
        }
    };
    let mut tcp = tokio::net::TcpStream::connect(downstream_addr)
        .await
        .context("failed to connect to downstream")?;
    let (mut downstream_read, mut downstream_write) = tcp.split();

    let mut upstream_to_downstream: BufferedCopy<{ 1024 * 64 }> = BufferedCopy::new();
    let mut downstream_to_upstream: BufferedCopy<{ 1024 * 64 }> = BufferedCopy::new();
    loop {
        tokio::select! {
            res = upstream_to_downstream.copy(&mut upstream_read, &mut downstream_write) => {
                if res.is_err() {
                    let _ = upstream_write.reset(p2proxy_lib::proto::GENERIC_QUIC_ERROR_CODE);
                    let _ = upstream_read.stop(p2proxy_lib::proto::GENERIC_QUIC_ERROR_CODE);
                }
                match res {
                    Err(BufCopyError::QuicClosed(c)) => {
                        tracing::debug!("Quic connection closed with code={c}");
                        return Ok(());
                    }
                    Err(BufCopyError::TCPEoF) => {
                        tracing::debug!("Tcp connection end of file");
                        return Ok(());
                    }
                    _ => {}
                }
                res?;
            }
            res = downstream_to_upstream.copy(&mut downstream_read, &mut upstream_write) => {
                if res.is_err() {
                    let _ = upstream_write.reset(p2proxy_lib::proto::GENERIC_QUIC_ERROR_CODE);
                    let _ = upstream_read.stop(p2proxy_lib::proto::GENERIC_QUIC_ERROR_CODE);
                }
                match res {
                    Err(BufCopyError::QuicClosed(c)) => {
                        tracing::debug!("Quic connection closed with code={c}");
                        return Ok(());
                    }
                    Err(BufCopyError::TCPEoF) => {
                        tracing::debug!("Tcp connection end of file");
                        return Ok(());
                    }
                    _ => {}
                }
                res?;
            }
        }
    }
}
