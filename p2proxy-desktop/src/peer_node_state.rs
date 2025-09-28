use crate::peer_id::PeerId;
use crate::{App, AppMessage};
use iced::Task;
use iced::futures::Stream;
use iced::widget::button;
use iced_core::Padding;
use iroh_base::NodeId;
use p2proxy_client::ServeUpdate;
use p2proxy_client::killswitch::ProxyKillSwitch;

use p2proxy_lib::display_chain;
use p2proxy_lib::proto::ServerPortMapString;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

#[derive(Default, Debug, Clone)]
pub(super) struct PeerNodeState {
    pub(super) peers: Vec<PeerNode>,
}

#[derive(Debug, Clone)]
pub(super) struct PeerNode {
    pub(super) peer_id: PeerId,
    node_id_input: String,
    node_parse_error: Option<String>,
    port_input: String,
    port_parse_error: Option<String>,
    node_id: Option<NodeId>,
    port: Option<u16>,
    rtt: Option<Duration>,
    con_err: Option<String>,
    con_state: Option<String>,
    named_port_toggled: bool,
    named_port: String,
    proxy_ready: bool,
    proxy_killswitch: Option<Arc<ProxyKillSwitch>>,
}

impl PeerNode {
    fn from_peer_id(peer_id: PeerId) -> Self {
        Self {
            peer_id,
            node_id_input: String::new(),
            node_parse_error: None,
            port_input: "8080".to_string(),
            port_parse_error: None,
            node_id: None,
            port: Some(8080),
            rtt: None,
            con_err: None,
            con_state: None,
            named_port_toggled: false,
            named_port: String::new(),
            proxy_ready: false,
            proxy_killswitch: None,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) enum PeerNodeStateMessage {
    AddSlot,
    NodeInput(PeerId, String),
    PortInput(PeerId, String),
    Ping(PeerId),
    PingResult(PeerId, Result<Duration, String>),
    NamedPortToggle(PeerId, bool),
    NamedPortInput(PeerId, String),
    Proxy(PeerId),
    StopProxy(PeerId),
    ConnectionReady(PeerId),
    ConUpdate(PeerId, String),
    ProxyDied(PeerId, String),
    PopBrowser(PeerId, u16),
}

struct PeerTcpNodeStream {
    peer_id: PeerId,
    recv: tokio::sync::mpsc::Receiver<anyhow::Result<ServeUpdate>>,
}

impl AppMessage {
    fn con_update(peer_id: PeerId, s: String) -> Self {
        Self::PeerNodeState(PeerNodeStateMessage::ConUpdate(peer_id, s))
    }
}

impl Stream for PeerTcpNodeStream {
    type Item = AppMessage;

    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let poll = self.recv.poll_recv(cx);
        let res = match poll {
            Poll::Ready(Some(Ok(upd))) => upd,
            Poll::Ready(Some(Err(e))) => {
                return Poll::Ready(Some(AppMessage::PeerNodeState(
                    PeerNodeStateMessage::ProxyDied(self.peer_id, display_chain(&*e).to_string()),
                )));
            }
            Poll::Ready(None) => return Poll::Ready(None),
            Poll::Pending => return Poll::Pending,
        };
        let msg = match res {
            ServeUpdate::BindingTcp => {
                AppMessage::con_update(self.peer_id, "binding tcp".to_string())
            }
            ServeUpdate::ListeningTcp => {
                AppMessage::PeerNodeState(PeerNodeStateMessage::ConnectionReady(self.peer_id))
            }
            ServeUpdate::AcceptedTcp(_o) => {
                AppMessage::con_update(self.peer_id, "accepted tcp".to_string())
            }
            ServeUpdate::IrohConnecting(_o) => {
                AppMessage::con_update(self.peer_id, "connecting".to_string())
            }
            ServeUpdate::ConnectionError(_o, e) => AppMessage::con_update(
                self.peer_id,
                format!("connection error: {}", display_chain(&*e)),
            ),
        };
        Poll::Ready(Some(msg))
    }
}

impl App {
    #[allow(clippy::too_many_lines)]
    pub(super) fn on_peer_node_state_message(
        &mut self,
        msg: PeerNodeStateMessage,
    ) -> Task<AppMessage> {
        let Some(has_key) = &mut self.has_key_state else {
            return Task::none();
        };
        match msg {
            PeerNodeStateMessage::AddSlot => {
                // Only add an empty slot if the last slot is non-empty (or missing)
                if let Some(last) = has_key.peer_node_state.peers.last()
                    && last.node_id.is_none()
                {
                    return Task::none();
                }
                has_key
                    .peer_node_state
                    .peers
                    .push(PeerNode::from_peer_id(self.peer_id_generator.next_id()));
            }
            PeerNodeStateMessage::NodeInput(n, raw) => {
                if let Some(peer) = has_key.peer_mut_by_id(n) {
                    if raw.len() == 64 {
                        match iroh_base::NodeId::from_str(&raw) {
                            Ok(o) => {
                                peer.node_parse_error = None;
                                peer.node_id = Some(o);
                            }
                            Err(e) => {
                                peer.node_parse_error = Some(display_chain(&e).to_string());
                                peer.node_id = None;
                            }
                        }
                    }
                    peer.node_id_input = raw;
                }
            }
            PeerNodeStateMessage::PortInput(n, raw) => {
                if let Some(peer) = has_key.peer_mut_by_id(n) {
                    peer.port_input = raw;
                    match peer.port_input.parse::<u16>() {
                        Ok(p) => {
                            peer.port_parse_error = None;
                            peer.port = Some(p);
                        }
                        Err(_e) => {
                            peer.port_parse_error =
                                Some(format!("invalid port {}", peer.port_input));
                        }
                    }
                }
            }
            PeerNodeStateMessage::Ping(n) => {
                if let Some(peer) = has_key.peer_mut_by_id(n)
                    && let Some(node_id) = peer.node_id
                {
                    let ep_c = has_key.endpoint.clone();
                    return Task::future(async move {
                        match p2proxy_client::exec_ping(&ep_c, node_id).await {
                            Ok(d) => AppMessage::PeerNodeState(PeerNodeStateMessage::PingResult(
                                n,
                                Ok(d),
                            )),
                            Err(e) => AppMessage::PeerNodeState(PeerNodeStateMessage::PingResult(
                                n,
                                Err(display_chain(&*e).to_string()),
                            )),
                        }
                    });
                }
            }
            PeerNodeStateMessage::PingResult(n, r) => {
                if let Some(peer) = has_key.peer_mut_by_id(n) {
                    match r {
                        Ok(o) => {
                            peer.rtt = Some(o);
                            peer.con_err = None;
                        }
                        Err(e) => {
                            peer.con_err = Some(e);
                        }
                    }
                }
            }
            PeerNodeStateMessage::NamedPortToggle(n, t) => {
                if let Some(peer) = has_key.peer_mut_by_id(n) {
                    peer.named_port_toggled = t;
                }
            }
            PeerNodeStateMessage::NamedPortInput(n, name) => {
                if let Some(peer) = has_key.peer_mut_by_id(n) {
                    peer.named_port = name;
                }
            }
            PeerNodeStateMessage::Proxy(n) => {
                if let Some(peer) = has_key.peer_mut_by_id(n)
                    && let Some(node_id) = peer.node_id
                    && let Some(port) = peer.port
                {
                    let spm = if peer.named_port_toggled {
                        let spm = match ServerPortMapString::try_new(peer.named_port.clone()) {
                            Ok(o) => o,
                            Err(e) => {
                                peer.con_err = Some(display_chain(&*e).to_string());
                                return Task::none();
                            }
                        };
                        Some(spm)
                    } else {
                        None
                    };
                    tracing::info!("proxying to {node_id} at {spm:?}");
                    let (kill, listen) = ProxyKillSwitch::new_pair();
                    peer.proxy_killswitch = Some(Arc::new(kill));
                    let updates = p2proxy_client::spawn_serve_with_updates_killswitched(
                        has_key.endpoint.clone(),
                        node_id,
                        port,
                        spm,
                        listen,
                    );

                    let stream = PeerTcpNodeStream {
                        peer_id: n,
                        recv: updates,
                    };
                    return Task::stream(stream);
                }
            }
            PeerNodeStateMessage::StopProxy(n) => {
                if let Some(peer) = has_key.peer_mut_by_id(n) {
                    peer.proxy_killswitch.take();
                    peer.proxy_ready = false;
                }
            }
            PeerNodeStateMessage::ProxyDied(n, s) => {
                if let Some(peer) = has_key.peer_mut_by_id(n) {
                    peer.con_err = Some(s);
                    peer.con_state = None;
                    peer.proxy_ready = false;
                    if let Some(ks) = peer.proxy_killswitch.take() {
                        ks.signal();
                    }
                }
            }
            PeerNodeStateMessage::ConUpdate(n, msg) => {
                if let Some(peer) = has_key.peer_mut_by_id(n) {
                    peer.con_state = Some(msg);
                    peer.con_err = None;
                }
            }
            PeerNodeStateMessage::ConnectionReady(n) => {
                if let Some(peer) = has_key.peer_mut_by_id(n) {
                    peer.con_state = Some("connected".to_string());
                    peer.proxy_ready = true;
                    peer.con_err = None;
                }
            }
            PeerNodeStateMessage::PopBrowser(n, port) => {
                if let Some(peer) = has_key.peer_mut_by_id(n)
                    && let Err(e) = opener::open_browser(format!("http://localhost:{port}"))
                {
                    peer.con_err = Some(format!("failed to open browser: {}", display_chain(&e)));
                }
            }
        }

        Task::none()
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn peer_node_state(&self) -> iced::widget::Column<'_, AppMessage> {
        let Some(has_key) = &self.has_key_state else {
            return iced::widget::Column::new();
        };
        let mut col = iced::widget::Column::new().push(
            button("Add node").on_press(AppMessage::PeerNodeState(PeerNodeStateMessage::AddSlot)),
        );

        for p in &has_key.peer_node_state.peers {
            let proxy_running = p.proxy_killswitch.is_some();
            let pid = p.peer_id;
            let mut start_btn = iced::widget::button("proxy");
            if p.node_id.is_some() && p.port.is_some() {
                start_btn =
                    start_btn.on_press(AppMessage::PeerNodeState(PeerNodeStateMessage::Proxy(pid)));
            }
            let mut stop_btn = iced::widget::button("stop");
            if proxy_running {
                stop_btn = stop_btn.on_press(AppMessage::PeerNodeState(
                    PeerNodeStateMessage::StopProxy(p.peer_id),
                ));
            }
            let mut proxy_row = iced::widget::row([
                iced::widget::row([
                    start_btn.into(),
                    iced::widget::Space::with_width(10.).into(),
                    stop_btn.style(iced::widget::button::danger).into(),
                    iced::widget::Space::with_width(25.).into(),
                ])
                .width(150.)
                .into(),
                iced::widget::text("Status: ")
                    .line_height(1.85)
                    .width(100.)
                    .into(),
                iced::widget::text(p.con_state.as_deref().unwrap_or("-"))
                    .line_height(1.85)
                    .into(),
            ])
            .padding(Padding::default().top(15.).bottom(15.));
            if let Some(e) = &p.con_err {
                proxy_row = proxy_row
                    .push(iced::widget::Space::with_width(25.))
                    .push(iced::widget::text(format!("Error: {e}")));
            }
            let node_col = iced::widget::column([
                iced::widget::row([
                    iced::widget::text("Peer:").line_height(1.85).into(),
                    iced::widget::horizontal_space().width(10.).into(),
                    iced::widget::text_input("peer node id", &p.node_id_input)
                        .on_input(move |s| {
                            AppMessage::PeerNodeState(PeerNodeStateMessage::NodeInput(p.peer_id, s))
                        })
                        .into(),
                    iced::widget::horizontal_space().width(10.).into(),
                    iced::widget::text("Port:").line_height(1.85).into(),
                    iced::widget::horizontal_space().width(10.).into(),
                    iced::widget::text_input("local tcp port", &p.port_input)
                        .on_input(move |s| {
                            AppMessage::PeerNodeState(PeerNodeStateMessage::PortInput(p.peer_id, s))
                        })
                        .width(75.)
                        .into(),
                ])
                .padding(Padding::default().top(20.).bottom(15.))
                .into(),
                iced::widget::row([
                    iced::widget::row([
                        iced::widget::button("ping")
                            .on_press_maybe(p.node_id.as_ref().map(|_| {
                                AppMessage::PeerNodeState(PeerNodeStateMessage::Ping(p.peer_id))
                            }))
                            .into(),
                        iced::widget::Space::with_width(25.).into(),
                    ])
                    .width(150.)
                    .into(),
                    iced::widget::text("Last RTT: ")
                        .line_height(1.85)
                        .width(100.)
                        .into(),
                    iced::widget::text(
                        p.rtt
                            .map_or_else(|| "-".to_string(), |d| format!("{}ms", d.as_millis())),
                    )
                    .line_height(1.85)
                    .into(),
                ])
                .into(),
                iced::widget::row([
                    iced::widget::checkbox("Use named port", p.named_port_toggled)
                        .text_line_height(1.85)
                        .on_toggle(|sel| {
                            AppMessage::PeerNodeState(PeerNodeStateMessage::NamedPortToggle(
                                p.peer_id, sel,
                            ))
                        })
                        .into(),
                    iced::widget::horizontal_space().width(10.).into(),
                    iced::widget::text_input("port name", &p.named_port)
                        .on_input_maybe(p.named_port_toggled.then_some(move |s| {
                            AppMessage::PeerNodeState(PeerNodeStateMessage::NamedPortInput(
                                p.peer_id, s,
                            ))
                        }))
                        .into(),
                ])
                .padding(Padding::default().top(20.).bottom(15.))
                .into(),
                proxy_row.into(),
                iced::widget::row([iced::widget::button("open")
                    .on_press_maybe(p.port.filter(|_p| p.proxy_ready).map(|port| {
                        AppMessage::PeerNodeState(PeerNodeStateMessage::PopBrowser(p.peer_id, port))
                    }))
                    .into()])
                .into(),
            ]);
            col = col.push(node_col);
        }
        col
    }
}
