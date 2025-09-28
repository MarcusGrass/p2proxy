mod build;
mod shell;
mod tests;

use crate::shell::RunningTestServer;
use build::BuiltBinaries;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let built_binaries = BuiltBinaries::build_all()?;
    let mut test_server_4501 = RunningTestServer::run_on_port(&built_binaries.test_server, 4501)?;
    let mut test_server_4502 = RunningTestServer::run_on_port(&built_binaries.test_server, 4502)?;
    let mut test_server_4503 = RunningTestServer::run_on_port(&built_binaries.test_server, 4503)?;
    let simple_res = tests::simple::connect_any(&built_binaries, 11991).await;
    let extensive_res =
        tests::extensive::extensive(&built_binaries, [11992, 11993, 11994, 11995, 11996, 11997])
            .await;
    if simple_res.is_err() || extensive_res.is_err() {
        eprintln!("Errors running test, server at 4501:");
        test_server_4501.dump_output().await;
        eprintln!("server at 4502:");
        test_server_4502.dump_output().await;
        eprintln!("server at 4503:");
        test_server_4503.dump_output().await;
    }

    Ok(())
}
