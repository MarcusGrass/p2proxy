use crate::api::tokens::{UserDefinedKey, UserDefinedNode};
use crate::frb_generated::StreamSink;
use flutter_rust_bridge::frb;
use iroh::Endpoint;
use iroh_base::NodeId;
use p2proxy_client::ServeUpdate;
use p2proxy_client::killswitch::ProxyKillSwitch;
use p2proxy_lib::display_chain;
use p2proxy_lib::proto::ServerPortMapString;
use std::sync::{Arc, Mutex};

pub struct InitializedEndpoint {
    inner: Arc<Mutex<InitializedEndpointInner>>,
}

struct InitializedEndpointInner {
    endpoint: Endpoint,
    open_stream_handle: Option<ProxyKillSwitch>,
}

impl InitializedEndpoint {
    pub async fn create(key: &UserDefinedKey) -> Result<InitializedEndpoint, String> {
        let endpoint = p2proxy_client::init_endpoint(key.private_key.clone())
            .await
            .map_err(|e| display_chain(&*e).to_string())?;
        Ok(InitializedEndpoint {
            inner: Arc::new(Mutex::new(InitializedEndpointInner {
                endpoint,
                open_stream_handle: None,
            })),
        })
    }

    pub async fn exec_ping(&self, address: &UserDefinedNode) -> Result<i64, String> {
        let ep = self.inner.lock().unwrap().endpoint.clone();
        let rtt = p2proxy_client::exec_ping(&ep, address.node_id)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rtt.as_millis() as i64)
    }
    pub async fn serve_remote_tcp(
        &self,
        port: &i32,
        address: &UserDefinedNode,
        named_port: Option<String>,
        sink: StreamSink<String>,
    ) -> Result<(), String> {
        let (kill, listen) = ProxyKillSwitch::new_pair();
        let recv = {
            let ep = {
                let mut lock = self.inner.lock().unwrap();
                if let Some(old) = lock.open_stream_handle.replace(kill) {
                    log::info!("Cancelling old stream");
                    old.signal();
                }
                lock.endpoint.clone()
            };
            let port_map = named_port
                .map(ServerPortMapString::try_new)
                .transpose()
                .map_err(|e| {
                    format!(
                        "named port is not a valid ServerPortMapString: {}",
                        display_chain(&*e)
                    )
                })?;
            let port = (*port)
                .try_into()
                .map_err(|_| "port is not a valid u16".to_string())?;
            p2proxy_client::spawn_serve_with_updates_killswitched(
                ep,
                address.node_id,
                port,
                port_map,
                listen,
            )
        };
        Self::stream_listen_task(sink, recv, address.node_id).await;
        Ok(())
    }

    async fn stream_listen_task(
        mut sink: StreamSink<String>,
        mut recv: tokio::sync::mpsc::Receiver<anyhow::Result<ServeUpdate>>,
        address: NodeId,
    ) {
        loop {
            let msg = recv.recv().await;
            let Some(msg) = msg else {
                log::warn!(
                    "Stream closed, sender dropped, exiting job at address={}",
                    address
                );
                return;
            };
            if !Self::handle_update(msg, &mut sink) {
                break;
            }
        }
    }

    fn handle_update(update: anyhow::Result<ServeUpdate>, sink: &mut StreamSink<String>) -> bool {
        let update = match update {
            Ok(res) => res,
            Err(e) => {
                let _ = sink.add(format!("e {}", display_chain(&*e)));
                return false;
            }
        };
        match update {
            // These three are unimportant for the app, no need to
            // waste CPU on them
            ServeUpdate::BindingTcp
            | ServeUpdate::AcceptedTcp(_)
            | ServeUpdate::IrohConnecting(_) => {}
            ServeUpdate::ListeningTcp => {
                if sink.add("s listening".to_string()).is_err() {
                    return false;
                }
            }
            ServeUpdate::ConnectionError(_, e) => {
                if sink.add(format!("e {}", display_chain(&*e))).is_err() {
                    return false;
                }
            }
        }
        true
    }

    #[frb(sync)]
    pub fn cancel_stream(&self) {
        log::info!("Pre acquire lock on cancel");
        if let Some(handle) = self.inner.lock().unwrap().open_stream_handle.take() {
            log::info!("Canceling stream");
            handle.signal();
        } else {
            log::info!("Received cancel_stream but no stream is open");
        }
    }

    pub async fn destroy(&self) {
        let ep = {
            let mut lock = self.inner.lock().unwrap();
            if let Some(handle) = lock.open_stream_handle.take() {
                handle.signal();
            }
            lock.endpoint.clone()
        };
        ep.close().await;
    }
}
