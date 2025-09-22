use crate::access_log::AccessLogHandle;
use crate::configuration::P2ProxydSetup;
use crate::proto::P2ProxyProto;
use anyhow::Context;
use iroh::protocol::Router;
use p2proxy_lib::display_chain;
use p2proxy_lib::proto::ALPN;

pub(super) async fn run_proxy(cfg: P2ProxydSetup) -> anyhow::Result<()> {
    let nid = cfg.secret_key.public();
    let endpoint = iroh::Endpoint::builder()
        .alpns(vec![ALPN.to_vec()])
        .discovery_n0()
        .secret_key(cfg.secret_key)
        .bind()
        .await
        .context("Failed to bind to endpoint")?;
    let access_log_handle = cfg.access_log_handle;
    let al_c = access_log_handle.clone();
    let proto = P2ProxyProto::new(cfg.routes, access_log_handle);
    tracing::info!("running service with node_id={nid}");
    let router = Router::builder(endpoint).accept(ALPN, proto).spawn();
    if let Err(e) = sighand_loop(al_c).await {
        tracing::error!("Error in sighand loop: {}", display_chain(&*e));
    }
    router
        .shutdown()
        .await
        .context("failed to shutdown router")?;
    Ok(())
}

async fn sighand_loop(al: AccessLogHandle) -> anyhow::Result<()> {
    #[cfg(target_family = "unix")]
    {
        run_sighand_loop(al).await
    }
    #[cfg(not(target_family = "unix"))]
    {
        async_noop().await;
        Ok(())
    }
}

#[cfg(target_family = "unix")]
async fn run_sighand_loop(access_log_handle: AccessLogHandle) -> anyhow::Result<()> {
    let mut signal = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
        .context("Failed to create hangup signal")?;
    loop {
        signal
            .recv()
            .await
            .context("Failed to receive hangup signal")?;
        access_log_handle.reload_file()?;
    }
}

#[inline]
#[cfg(not(target_family = "unix"))]
async fn async_noop() {
    futures::future::pending::<()>().await;
}
