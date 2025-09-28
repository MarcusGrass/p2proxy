use crate::build::BuiltBinaries;
use crate::shell::{RunningCli, RunningP2Proxyd};
use crate::tests::{try_http_big_payload, try_http_hello_world};
use iroh::NodeId;
use std::str::FromStr;

pub async fn connect_any(built_binaries: &BuiltBinaries, local_port: u16) -> anyhow::Result<()> {
    let mut p2proxyd_simple =
        RunningP2Proxyd::run_with_cfg(&built_binaries.p2proxyd, "./assets/config/simple.toml")?;

    let any_user_key = iroh::SecretKey::from_bytes(&[0u8; 32]);
    let simple_peer =
        NodeId::from_str("7c32ab7cdd9a4e2651c9eff072958a43a9b411cc5b69603a1dca6d7d843f2406")?;
    let mut cli_simple = RunningCli::run_with_key(
        &built_binaries.p2proxy_cli,
        &any_user_key,
        local_port,
        &simple_peer,
        None,
    )?;
    match try_http_hello_world(
        "simple::connect_any hello world",
        &format!("http://0.0.0.0:{}", cli_simple.port),
    )
    .await
    {
        Ok(()) => {
            println!("[simple::connect_any hello world] PASSED");
        }
        Err(e) => {
            eprintln!("{e}");
            p2proxyd_simple.dump_output().await;
            cli_simple.dump_output().await;
        }
    }
    match try_http_big_payload(
        "simple::connect_any big payload",
        &format!("http://0.0.0.0:{}", cli_simple.port),
    )
    .await
    {
        Ok(()) => {
            println!("[simple::connect_any big payload] PASSED");
        }
        Err(e) => {
            eprintln!("{e}");
            p2proxyd_simple.dump_output().await;
            cli_simple.dump_output().await;
        }
    }
    Ok(())
}
