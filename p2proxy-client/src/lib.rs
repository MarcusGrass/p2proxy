pub mod killswitch;

use crate::killswitch::{KillSwitchResult, ProxyKillSwitchListener};
use anyhow::{Context, bail};
use iroh::endpoint::Connection;
use iroh::{Endpoint, NodeAddr, NodeId, SecretKey};
use p2proxy_lib::display_chain;
use p2proxy_lib::proto::{ALPN, ServerPortMapString};
use p2proxy_lib::proxy_copy_buf::{BufCopyError, BufferedCopy};
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::str::FromStr;
use std::time::{Duration, Instant};
use tokio::net::{TcpListener, TcpStream};

#[inline]
pub fn generate_secret_key() -> SecretKey {
    iroh::SecretKey::generate(&mut rand::rngs::OsRng)
}

pub fn load_secret_key(key_path: &Path) -> anyhow::Result<SecretKey> {
    let key_material = std::fs::read(key_path)
        .with_context(|| format!("failed to read key from {}", key_path.display()))?;

    if key_material.len() == 32 {
        let raw: [u8; 32] = key_material
            .clone()
            .try_into()
            .map_err(|_e| anyhow::anyhow!("failed to parse key material"))?;
        Ok(iroh::SecretKey::from_bytes(&raw))
    } else if key_material.len() == 64 {
        // Hex
        let s =
            String::from_utf8(key_material).context("failed to parse key material, not utf8")?;
        iroh::SecretKey::from_str(&s).context("failed to parse key material")
    } else {
        bail!("invalid key material length, expected raw 32 bytes or hex string (64 bytes)")
    }
}

pub async fn init_endpoint(key: SecretKey) -> anyhow::Result<Endpoint> {
    iroh::Endpoint::builder()
        .discovery_n0()
        .secret_key(key)
        .bind()
        .await
        .context("failed to bind endpoint")
}

pub async fn exec_ping(endpoint: &Endpoint, peer: NodeId) -> anyhow::Result<Duration> {
    const PONG: &[u8] = b"PONG";
    let node_addr = NodeAddr::new(peer);
    let con = endpoint
        .connect(node_addr, ALPN)
        .await
        .with_context(|| format!("failed to connect to peer at {peer}"))?;
    let (mut send, mut recv) = con
        .open_bi()
        .await
        .with_context(|| format!("failed to open bi stream to peer at {peer}"))?;
    let sent = Instant::now();
    send.write_all(p2proxy_lib::proto::PING)
        .await
        .with_context(|| format!("failed to write ping to peer at {peer}"))?;
    let mut recv_buf = *b"PONG";
    recv.read_exact(&mut recv_buf)
        .await
        .with_context(|| format!("failed to read pong from peer at {peer}"))?;
    let elapsed = sent.elapsed();
    if recv_buf != PONG {
        bail!("expected pong, got {}", String::from_utf8_lossy(&recv_buf))
    }
    Ok(elapsed)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConId(u64);

impl Display for ConId {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug)]
pub enum ServeUpdate {
    BindingTcp,
    ListeningTcp,
    AcceptedTcp(ConId),
    IrohConnecting(ConId),
    ConnectionError(ConId, anyhow::Error),
}

#[must_use]
pub fn spawn_serve_with_updates_killswitched(
    endpoint: Endpoint,
    peer: NodeId,
    port: u16,
    dest_port_map: Option<ServerPortMapString>,
    mut kill_switch: ProxyKillSwitchListener,
) -> tokio::sync::mpsc::Receiver<anyhow::Result<ServeUpdate>> {
    let (send, recv) = tokio::sync::mpsc::channel(64);

    tokio::spawn(async move {
        let Some(ks_c) = kill_switch.duplicate() else {
            tracing::warn!("received kill signal before endpoint was spawned");
            return;
        };
        match kill_switch
            .if_not_killed(drive_tcp_task(
                send,
                port,
                dest_port_map,
                endpoint,
                peer,
                ks_c,
            ))
            .await
        {
            KillSwitchResult::Killed => {
                tracing::info!("proxy at {peer} on port {port} was killed, exiting proxy task");
            }
            KillSwitchResult::Finished(()) => {
                tracing::info!("proxy at {peer} on port {port} task completed, closing endpoint");
            }
        }
    });
    recv
}

async fn drive_tcp_task(
    send: tokio::sync::mpsc::Sender<anyhow::Result<ServeUpdate>>,
    local_port: u16,
    dest_port_map: Option<ServerPortMapString>,
    endpoint: Endpoint,
    peer: NodeId,
    mut proxy_kill_switch_listener: ProxyKillSwitchListener,
) {
    {
        let addr = format!("0.0.0.0:{local_port}");
        if send.try_send(Ok(ServeUpdate::BindingTcp)).is_err() {
            tracing::warn!("failed to send binding update");
            return;
        }
        tracing::info!("binding tcp socket at {addr}");
        let tcp = match TcpListener::bind(&addr).await {
            Ok(o) => o,
            Err(e) => {
                let _ = send.try_send(Err(anyhow::anyhow!(
                    "failed to bind tcp socket at {addr}: {}",
                    display_chain(&e)
                )));
                tracing::warn!("failed to bind tcp socket at {addr}: {}", display_chain(&e));
                return;
            }
        };
        if send.try_send(Ok(ServeUpdate::ListeningTcp)).is_err() {
            tracing::warn!("failed to send listening TCP");
            return;
        }
        let mut con_count = 0u64;
        loop {
            tokio::select! {
                () = proxy_kill_switch_listener.killed() => {
                    tracing::info!("received kill signal, exiting tcp task");
                    return;
                }
                tcp_res = tcp.accept() => {
                    let next = match tcp_res {
                        Ok((stream, _addr)) => stream,
                        Err(e) => {
                            if send
                                .try_send(Err(anyhow::anyhow!(
                                    "failed to accept tcp connection: {}",
                                    display_chain(&e)
                                ))).is_err() {
                                tracing::warn!("failed to send tcp accept error");
                            }
                            return;
                        }
                    };
                    con_count += 1;
                    let con_id = ConId(con_count);
                    tracing::debug!("accepted tcp connection for con_id={con_id}");
                    let Some(ks_c) = proxy_kill_switch_listener.duplicate() else {
                        tracing::info!("received kill signal before connection was spawned");
                        return;
                    };
                    tokio::task::spawn(run_on_tcp(endpoint.clone(), con_id, next, peer, dest_port_map.clone(), send.clone(), ks_c));
                }
                () = send.closed() => {
                    tracing::debug!("updates receiver dropped");
                    return;
                }
            }
        }
    }
}

#[allow(clippy::too_many_lines)]
async fn run_on_tcp(
    endpoint: Endpoint,
    con_id: ConId,
    mut tcp: TcpStream,
    peer: NodeId,
    dest_port_map: Option<ServerPortMapString>,
    sender: tokio::sync::mpsc::Sender<anyhow::Result<ServeUpdate>>,
    mut proxy_kill_switch_listener: ProxyKillSwitchListener,
) {
    const CONNECTION_LIVE_AFTER: Duration = Duration::from_secs(2);
    let node_addr = NodeAddr::new(peer);
    let mut failed_connects = 0;

    loop {
        tracing::debug!("running quic connection loop, failed_reconnects = {failed_connects}");
        if failed_connects >= 3 {
            let _ = sender
                .send(Err(anyhow::anyhow!(
                    "Giving up on connection after {failed_connects} attempts"
                )))
                .await;
            return;
        }
        if sender
            .send(Ok(ServeUpdate::IrohConnecting(con_id)))
            .await
            .is_err()
        {
            return;
        }
        let KillSwitchResult::Finished(con_res) = proxy_kill_switch_listener
            .if_not_killed(tokio::time::timeout(
                Duration::from_millis(10_000),
                endpoint.connect(node_addr.clone(), ALPN),
            ))
            .await
        else {
            tracing::info!("received kill signal, exiting quic connection task");
            return;
        };
        match con_res {
            Ok(Ok(con)) => {
                let con_start = Instant::now();
                if let Err(e) = run_connection(
                    con,
                    &mut tcp,
                    dest_port_map.as_ref(),
                    &mut proxy_kill_switch_listener,
                )
                .await
                {
                    match e {
                        BufCopyError::QuicConnectionForbidden
                        | BufCopyError::QuicStreamForbidden => {
                            let _ = sender.try_send(Err(anyhow::anyhow!(
                                "quic connection forbidden at {}",
                                dest_port_map.as_ref().map_or(
                                    "default path",
                                    p2proxy_lib::proto::ServerPortMapString::as_str
                                )
                            )));
                            tracing::warn!(
                                "quic connection forbidden at {}",
                                dest_port_map.as_ref().map_or(
                                    "default path",
                                    p2proxy_lib::proto::ServerPortMapString::as_str
                                )
                            );
                            // Don't retry on forbidden
                            return;
                        }
                        BufCopyError::TCPEoF => {
                            tracing::debug!("Tcp EOF, shutting down connection");
                            // Connection is complete, this is not necessarily
                            // an error (although it could theoretically be)
                            return;
                        }
                        BufCopyError::QuicClosed(_)
                        | BufCopyError::QuicInternal
                        | BufCopyError::Unactionable(_) => {}
                    }
                    if sender
                        .try_send(Ok(ServeUpdate::ConnectionError(
                            con_id,
                            anyhow::anyhow!("connection failed: {}", display_chain(&e)),
                        )))
                        .is_err()
                    {
                        tracing::warn!("failed to send connection error: {}", display_chain(&e));
                        return;
                    }
                    tracing::warn!("connection failed: {}", display_chain(&e));
                    // Treating a short-lived connection heuristically as a connection failure.
                    // If a connection is rejected on authorization, the connection will succeed but
                    // any data-transfer will fail. Thus, this loop will spam if unhandled.
                    let elapsed = con_start.elapsed();
                    if con_start.elapsed() < CONNECTION_LIVE_AFTER {
                        let wait = CONNECTION_LIVE_AFTER.saturating_sub(elapsed);
                        tokio::time::sleep(wait).await;
                        tracing::debug!("Sleeping for {wait:?} before retrying connection");
                        failed_connects += 1;
                    } else {
                        failed_connects = 0;
                    }
                }
            }
            Ok(Err(e)) => {
                failed_connects += 1;
                if sender
                    .try_send(Err(anyhow::anyhow!(
                        "failed to connect to peer on attempt={failed_connects}: {}",
                        display_chain(&e)
                    )))
                    .is_err()
                {
                    tracing::warn!(
                        "failed to send peer failed to connect after {failed_connects} attempts: {}",
                        display_chain(&e)
                    );
                    return;
                }
                tracing::warn!(
                    "failed to connect to peer on attempt={failed_connects}: {}",
                    display_chain(&e)
                );
            }
            Err(_e) => {
                let _ = sender.try_send(Err(anyhow::anyhow!("failed to connect to peer timeout",)));
                tracing::warn!("failed to connect to peer, timeout");
                return;
            }
        }
    }
}
async fn run_connection(
    downstream_connection: Connection,
    tcp: &mut TcpStream,
    dest_port_map: Option<&ServerPortMapString>,
    proxy_kill_switch_listener: &mut ProxyKillSwitchListener,
) -> Result<(), BufCopyError> {
    let (mut upstream_read, mut upstream_write) = tcp.split();
    let (mut downstream_write, mut downstream_read) = downstream_connection
        .open_bi()
        .await
        .context("failed to accept downstream connection")?;
    let payload = if let Some(dpm) = dest_port_map {
        dpm.as_bytes()
    } else {
        p2proxy_lib::proto::DEFAULT_ROUTE
    };
    downstream_write
        .write_all(payload)
        .await
        .context("failed to write hello to upstream")?;
    let mut upstream_to_downstream: BufferedCopy<{ 1024 * 64 }> = BufferedCopy::new();
    let mut downstream_to_upstream: BufferedCopy<{ 1024 * 64 }> = BufferedCopy::new();
    loop {
        tokio::select! {
            res = upstream_to_downstream.copy(&mut upstream_read, &mut downstream_write) => {
                if res.is_err() {
                    let _ = downstream_write.reset(p2proxy_lib::proto::GENERIC_QUIC_ERROR_CODE);
                    let _ = downstream_read.stop(p2proxy_lib::proto::GENERIC_QUIC_ERROR_CODE);
                }
                res?;
            }
            res = downstream_to_upstream.copy(&mut downstream_read, &mut upstream_write) => {
                if res.is_err() {
                    let _ = downstream_write.reset(p2proxy_lib::proto::GENERIC_QUIC_ERROR_CODE);
                    let _ = downstream_read.stop(p2proxy_lib::proto::GENERIC_QUIC_ERROR_CODE);
                }
                res?;
            }
            () = proxy_kill_switch_listener.killed() => {
                let _ = downstream_write.finish();
                let _ = downstream_read.stop(p2proxy_lib::proto::QUIC_OK_ERROR_CODE);
                tracing::info!("received kill signal, exiting connection task");
                return Ok(());
            }
        }
    }
}
