use anyhow::Context;
use iroh::NodeId;
use p2proxy_lib::display_chain;
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

#[derive(Debug, Clone)]
pub struct AccessLogHandle {
    chan: Option<std::sync::mpsc::SyncSender<AccessLogWriterMessage>>,
}

pub enum AccessLogWriterMessage {
    IncomingConnection(IncomingConnection),
    ReloadFile,
}

impl AccessLogHandle {
    pub fn maybe_spawn(path: Option<PathBuf>) -> Self {
        if let Some(path) = path {
            let (chan, receiver) = std::sync::mpsc::sync_channel(100);
            std::thread::spawn(move || access_log_writer_outer_loop(&receiver, &path));
            Self { chan: Some(chan) }
        } else {
            Self { chan: None }
        }
    }

    pub fn log_rejected_missing_node_id(&self, address: SocketAddr) -> anyhow::Result<()> {
        let Some(chan) = &self.chan else {
            return Ok(());
        };
        chan.try_send(AccessLogWriterMessage::IncomingConnection(
            IncomingConnection {
                timestamp: timestamp_try_local_offset(),
                address,
                result: IncomingConnectionResult::MissingNodeId,
            },
        ))
        .context("failed to send rejected missing node to incoming connection log channel")
    }

    pub fn log_rejected_not_allowed_at(
        &self,
        address: SocketAddr,
        node_id: NodeId,
        port: String,
    ) -> anyhow::Result<()> {
        let Some(chan) = &self.chan else {
            return Ok(());
        };
        chan.try_send(AccessLogWriterMessage::IncomingConnection(
            IncomingConnection {
                timestamp: timestamp_try_local_offset(),
                address,
                result: IncomingConnectionResult::RejectedNotAllowedPort(node_id, port),
            },
        ))
        .context("failed to send rejected to incoming connection log channel")
    }

    pub fn log_rejected_default_not_present(
        &self,
        address: SocketAddr,
        node_id: NodeId,
    ) -> anyhow::Result<()> {
        let Some(chan) = &self.chan else {
            return Ok(());
        };
        chan.try_send(AccessLogWriterMessage::IncomingConnection(
            IncomingConnection {
                timestamp: timestamp_try_local_offset(),
                address,
                result: IncomingConnectionResult::RejectedDefaultRoute(node_id),
            },
        ))
        .context("failed to send rejected default port to incoming connection log channel")
    }

    pub fn log_rejected_unknown_port_mapping(
        &self,
        address: SocketAddr,
        node_id: NodeId,
        mapping: String,
    ) -> anyhow::Result<()> {
        let Some(chan) = &self.chan else {
            return Ok(());
        };
        chan.try_send(AccessLogWriterMessage::IncomingConnection(
            IncomingConnection {
                timestamp: timestamp_try_local_offset(),
                address,
                result: IncomingConnectionResult::RejectedUnknownPortMapping(node_id, mapping),
            },
        ))
        .context("failed to send rejected unknown port mapping to incoming connection log channel")
    }

    pub fn log_rejected_garbage_port_mapping(
        &self,
        address: SocketAddr,
        node_id: NodeId,
        mapping: [u8; 16],
    ) -> anyhow::Result<()> {
        let Some(chan) = &self.chan else {
            return Ok(());
        };
        chan.try_send(AccessLogWriterMessage::IncomingConnection(
            IncomingConnection {
                timestamp: timestamp_try_local_offset(),
                address,
                result: IncomingConnectionResult::RejectedGarbagePortMapping(node_id, mapping),
            },
        ))
        .context("failed to send rejected garbage port mapping to incoming connection log channel")
    }

    pub fn log_accepted(&self, address: SocketAddr, node_id: NodeId) -> anyhow::Result<()> {
        let Some(chan) = &self.chan else {
            return Ok(());
        };
        chan.try_send(AccessLogWriterMessage::IncomingConnection(
            IncomingConnection {
                timestamp: timestamp_try_local_offset(),
                address,
                result: IncomingConnectionResult::Accepted(node_id),
            },
        ))
        .context("failed to send accepted to incoming connection log channel")
    }

    pub fn reload_file(&self) -> anyhow::Result<()> {
        let Some(chan) = &self.chan else {
            return Ok(());
        };
        chan.try_send(AccessLogWriterMessage::ReloadFile)
            .context("failed to send reload file to incoming connection log channel")
    }
}

// It's generally easier to read logs in local offset (in my opinion).
// So try getting the local offset first, if that doesn't work, use UTC
fn timestamp_try_local_offset() -> OffsetDateTime {
    OffsetDateTime::now_local()
        .ok()
        .unwrap_or_else(OffsetDateTime::now_utc)
}

pub struct IncomingConnection {
    timestamp: time::OffsetDateTime,
    address: SocketAddr,
    result: IncomingConnectionResult,
}

enum IncomingConnectionResult {
    MissingNodeId,
    Accepted(NodeId),
    RejectedGarbagePortMapping(NodeId, [u8; 16]),
    RejectedUnknownPortMapping(NodeId, String),
    RejectedNotAllowedPort(NodeId, String),
    RejectedDefaultRoute(NodeId),
}

fn access_log_writer(
    chan: &std::sync::mpsc::Receiver<AccessLogWriterMessage>,
    mut file: std::fs::File,
) -> anyhow::Result<()> {
    while let Ok(msg) = chan.recv() {
        let conn = match msg {
            AccessLogWriterMessage::IncomingConnection(c) => c,
            AccessLogWriterMessage::ReloadFile => {
                tracing::info!("Reloading access log file");
                return Ok(());
            }
        };
        match conn.result {
            IncomingConnectionResult::MissingNodeId => {
                if let Err(e) = file.write_fmt(format_args!(
                    "{}\t[{}]\tREJECTED\tCould not extract node id\n",
                    conn.timestamp
                        .format(&Rfc3339)
                        .context("failed to format timestamp")?,
                    conn.address
                )) {
                    tracing::error!("Failed to write to access log file: {}", display_chain(&e));
                    return Ok(());
                }
            }
            IncomingConnectionResult::RejectedGarbagePortMapping(node, port_mapping) => {
                if let Err(e) = file.write_fmt(format_args!(
                    "{}\t[{}]\t{node}\tREJECTED\tNode attempted un-parseable port mapping: '{}'\n",
                    conn.timestamp
                        .format(&Rfc3339)
                        .context("failed to format timestamp")?,
                    conn.address,
                    String::from_utf8_lossy(&port_mapping)
                )) {
                    tracing::error!("Failed to write to access log file: {}", display_chain(&e));
                    return Ok(());
                }
            }
            IncomingConnectionResult::RejectedUnknownPortMapping(node, port_mapping) => {
                if let Err(e) = file.write_fmt(format_args!(
                    "{}\t[{}]\t{node}\tREJECTED\tNode attempted missing port map: '{port_mapping}'\n",
                    conn.timestamp
                        .format(&Rfc3339)
                        .context("failed to format timestamp")?,
                    conn.address
                )) {
                    tracing::error!("Failed to write to access log file: {}", display_chain(&e));
                    return Ok(());
                }
            }
            IncomingConnectionResult::RejectedNotAllowedPort(node, port_mapping) => {
                if let Err(e) = file.write_fmt(format_args!(
                    "{}\t[{}]\t{node}\tREJECTED\tNode not approved for port map: '{port_mapping}'\n",
                    conn.timestamp
                        .format(&Rfc3339)
                        .context("failed to format timestamp")?,
                    conn.address
                )) {
                    tracing::error!("Failed to write to access log file: {}", display_chain(&e));
                    return Ok(());
                }
            }
            IncomingConnectionResult::RejectedDefaultRoute(node) => {
                if let Err(e) = file.write_fmt(format_args!(
                    "{}\t[{}]\t{node}\tREJECTED\tNode wanted missing default route\n",
                    conn.timestamp
                        .format(&Rfc3339)
                        .context("failed to format timestamp")?,
                    conn.address
                )) {
                    tracing::error!("Failed to write to access log file: {}", display_chain(&e));
                    return Ok(());
                }
            }
            IncomingConnectionResult::Accepted(node) => {
                if let Err(e) = file.write_fmt(format_args!(
                    "{}\t[{}]\t{node}\tACCEPTED\tNode connected\n",
                    conn.timestamp
                        .format(&Rfc3339)
                        .context("failed to format timestamp")?,
                    conn.address
                )) {
                    tracing::error!("Failed to write to access log file: {}", display_chain(&e));
                    return Ok(());
                }
            }
        }
    }
    Err(anyhow::anyhow!("incoming access log writer channel died"))
}

fn access_log_writer_outer_loop(
    chan: &std::sync::mpsc::Receiver<AccessLogWriterMessage>,
    path: &Path,
) {
    loop {
        match try_file(path) {
            Ok(o) => {
                if let Err(e) = access_log_writer(chan, o) {
                    tracing::error!("Failed to write to access log file: {}", display_chain(&*e));
                    return;
                }
            }
            Err(e) => {
                tracing::error!("Failed to open access log file: {}", display_chain(&*e));
                std::thread::sleep(std::time::Duration::from_secs(15));
            }
        }
    }
}

fn try_file(path: &Path) -> anyhow::Result<std::fs::File> {
    tracing::debug!("Opening access log file: {}", path.display());
    std::fs::File::options()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("Failed to open access log file: {}", path.display()))
}
