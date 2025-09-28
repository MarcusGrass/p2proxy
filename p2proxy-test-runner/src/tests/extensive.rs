use crate::build::BuiltBinaries;
use crate::shell::{RunningCli, RunningP2Proxyd};
use crate::tests::{try_any_http_expect_fail, try_http_hello_world};
use anyhow::Context;
use iroh::NodeId;
use std::str::FromStr;

pub async fn extensive(
    built_binaries: &BuiltBinaries,
    local_ports: [u16; 6],
) -> anyhow::Result<()> {
    let mut p2proxyd_extensive =
        RunningP2Proxyd::run_with_cfg(&built_binaries.p2proxyd, "./assets/config/extensive.toml")?;
    let extensive_pk =
        NodeId::from_str("f2b1ce018dda1d4e75d97fc9f86ecf30adbb0aba0977445ae85283502a8cc7be")?;
    extensive_any_can_connect_to_default(
        built_binaries,
        &mut p2proxyd_extensive,
        local_ports[0],
        &extensive_pk,
    )
    .await?;
    extensive_any_can_connect_to_demo_named_port(
        built_binaries,
        &mut p2proxyd_extensive,
        local_ports[1],
        &extensive_pk,
    )
    .await?;
    extensive_any_cant_connect_to_private(
        built_binaries,
        &mut p2proxyd_extensive,
        local_ports[2],
        &extensive_pk,
    )
    .await?;
    extensive_private_can_connect_to_private_port(
        built_binaries,
        &mut p2proxyd_extensive,
        local_ports[3],
        &extensive_pk,
    )
    .await?;
    extensive_superuser_can_connect_to_private_port(
        built_binaries,
        &mut p2proxyd_extensive,
        local_ports[4],
        &extensive_pk,
    )
    .await?;

    Ok(())
}

async fn extensive_any_can_connect_to_default(
    built_binaries: &BuiltBinaries,
    running_p2proxyd: &mut RunningP2Proxyd,
    local_port: u16,
    peer: &NodeId,
) -> anyhow::Result<()> {
    let any_user_key = iroh::SecretKey::generate(&mut rand::rngs::OsRng);
    let mut cli_extensive_unknown_node = RunningCli::run_with_key(
        &built_binaries.p2proxy_cli,
        &any_user_key,
        local_port,
        peer,
        None,
    )?;
    match try_http_hello_world(
        "extensive::connect_any_default",
        &format!("http://0.0.0.0:{}", cli_extensive_unknown_node.port),
    )
    .await
    {
        Ok(()) => {
            println!("[extensive::connect_any_default] PASSED");
        }
        Err(e) => {
            eprintln!("{e}");
            running_p2proxyd.dump_output().await;
            cli_extensive_unknown_node.dump_output().await;
        }
    }
    Ok(())
}

async fn extensive_any_can_connect_to_demo_named_port(
    built_binaries: &BuiltBinaries,
    running_p2proxyd: &mut RunningP2Proxyd,
    local_port: u16,
    peer: &NodeId,
) -> anyhow::Result<()> {
    let any_user_key = iroh::SecretKey::generate(&mut rand::rngs::OsRng);
    let mut cli_extensive_unknown_node = RunningCli::run_with_key(
        &built_binaries.p2proxy_cli,
        &any_user_key,
        local_port,
        peer,
        Some("demo"),
    )?;
    match try_http_hello_world(
        "extensive::connect_demo_named_port",
        &format!("http://0.0.0.0:{}", cli_extensive_unknown_node.port),
    )
    .await
    {
        Ok(()) => {
            println!("[extensive::connect_demo_named_port] PASSED");
        }
        Err(e) => {
            eprintln!("{e}");
            running_p2proxyd.dump_output().await;
            cli_extensive_unknown_node.dump_output().await;
        }
    }
    Ok(())
}

async fn extensive_any_cant_connect_to_private(
    built_binaries: &BuiltBinaries,
    running_p2proxyd: &mut RunningP2Proxyd,
    local_port: u16,
    peer: &NodeId,
) -> anyhow::Result<()> {
    let any_user_key = iroh::SecretKey::generate(&mut rand::rngs::OsRng);
    let mut cli_extensive_unknown_node = RunningCli::run_with_key(
        &built_binaries.p2proxy_cli,
        &any_user_key,
        local_port,
        peer,
        Some("private"),
    )?;
    match try_any_http_expect_fail(
        "extensive::connect_any_cant_connect_private",
        &format!("http://0.0.0.0:{}", cli_extensive_unknown_node.port),
    )
    .await
    {
        Ok(()) => {
            println!("[extensive::connect_any_cant_connect_private] PASSED");
        }
        Err(e) => {
            eprintln!("{e}");
            running_p2proxyd.dump_output().await;
            cli_extensive_unknown_node.dump_output().await;
        }
    }
    Ok(())
}

async fn extensive_private_can_connect_to_private_port(
    built_binaries: &BuiltBinaries,
    running_p2proxyd: &mut RunningP2Proxyd,
    local_port: u16,
    peer: &NodeId,
) -> anyhow::Result<()> {
    let any_user_key = private_access_key()?;
    let mut cli_extensive_unknown_node = RunningCli::run_with_key(
        &built_binaries.p2proxy_cli,
        &any_user_key,
        local_port,
        peer,
        Some("private"),
    )?;
    match try_http_hello_world(
        "extensive::connect_private_port",
        &format!("http://0.0.0.0:{}", cli_extensive_unknown_node.port),
    )
    .await
    {
        Ok(()) => {
            println!("[extensive::connect_private_port] PASSED");
        }
        Err(e) => {
            eprintln!("{e}");
            running_p2proxyd.dump_output().await;
            cli_extensive_unknown_node.dump_output().await;
        }
    }
    Ok(())
}

async fn extensive_superuser_can_connect_to_private_port(
    built_binaries: &BuiltBinaries,
    running_p2proxyd: &mut RunningP2Proxyd,
    local_port: u16,
    peer: &NodeId,
) -> anyhow::Result<()> {
    let any_user_key = superuser_key()?;
    let mut cli_extensive_unknown_node = RunningCli::run_with_key(
        &built_binaries.p2proxy_cli,
        &any_user_key,
        local_port,
        peer,
        Some("private"),
    )?;
    match try_http_hello_world(
        "extensive::connect_superuser_private_port",
        &format!("http://0.0.0.0:{}", cli_extensive_unknown_node.port),
    )
    .await
    {
        Ok(()) => {
            println!("[extensive::connect_superuser_private_port] PASSED");
        }
        Err(e) => {
            eprintln!("{e}");
            running_p2proxyd.dump_output().await;
            cli_extensive_unknown_node.dump_output().await;
        }
    }
    Ok(())
}

fn private_access_key() -> anyhow::Result<iroh::SecretKey> {
    let bytes = hex::decode("145f5e72f8c9fd9173a1b22538a2ecdc2f4a4022428e1029fbb08ca6fa0a785b")
        .context("failed to decode private access key hex")?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_e| anyhow::anyhow!("private access key hex is incorrect length"))?;
    Ok(iroh::SecretKey::from_bytes(&bytes))
}

fn superuser_key() -> anyhow::Result<iroh::SecretKey> {
    let bytes = hex::decode("7b66eb129bc5803e90c5afd816f98b865e78ec8aa56042fa6be6e8c415f377a1")
        .context("failed to decode private access key hex")?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_e| anyhow::anyhow!("private access key hex is incorrect length"))?;
    Ok(iroh::SecretKey::from_bytes(&bytes))
}
