use anyhow::bail;
use std::time::Duration;

pub mod extensive;
pub mod simple;

#[allow(clippy::cast_possible_truncation)]
async fn try_http_hello_world(label: &str, base_url: &str) -> anyhow::Result<()> {
    const MAX_ATTEMPTS: usize = 3;
    tokio::time::sleep(Duration::from_millis(200)).await;
    for i in 1..=MAX_ATTEMPTS {
        let response = match reqwest::get(base_url).await {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("[{label}] failed to get {base_url} on attempt {i}: {e}");
                let sleep_for = 200u32 * 2u32.pow(i as u32);
                tokio::time::sleep(Duration::from_millis(u64::from(sleep_for))).await;
                continue;
            }
        };
        match response.text().await {
            Ok(text) => {
                if text != "Hello, World!" {
                    bail!("[{label}] got an unexpected response: {text}")
                }
                return Ok(());
            }
            Err(e) => {
                eprintln!("[{label}] failed to get body on attempt {i}: {e}");
                let sleep_for = 200u32 * 2u32.pow(i as u32);
                tokio::time::sleep(Duration::from_millis(u64::from(sleep_for))).await;
            }
        }
    }
    bail!("[{label}] failed to successfully send request after {MAX_ATTEMPTS} attempts")
}

#[allow(clippy::cast_possible_truncation)]
async fn try_http_big_payload(label: &str, base_url: &str) -> anyhow::Result<()> {
    const MAX_ATTEMPTS: usize = 3;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let url = format!("{base_url}/big");
    for i in 1..=MAX_ATTEMPTS {
        let response = match reqwest::get(&url).await {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("[{label}] failed to get {url} on attempt {i}: {e}");
                let sleep_for = 200u32 * 2u32.pow(i as u32);
                tokio::time::sleep(Duration::from_millis(u64::from(sleep_for))).await;
                continue;
            }
        };
        match response.text().await {
            Ok(text) => {
                if !is_big_message(&text) {
                    bail!("[{label}] got an unexpected response: {text}")
                }
                return Ok(());
            }
            Err(e) => {
                eprintln!("[{label}] failed to get body on attempt {i}: {e}");
                let sleep_for = 200u32 * 2u32.pow(i as u32);
                tokio::time::sleep(Duration::from_millis(u64::from(sleep_for))).await;
            }
        }
    }
    bail!("[{label}] failed to successfully send request after {MAX_ATTEMPTS} attempts")
}

fn is_big_message(msg: &str) -> bool {
    let mut expect = String::with_capacity(2 * 1024 * 1024);
    for _i in 0..1024 * 1024 / 8 {
        expect.push_str("8-bytepl");
    }
    msg == expect
}

#[allow(clippy::cast_possible_truncation)]
async fn try_any_http_expect_fail(label: &str, url: &str) -> anyhow::Result<()> {
    const MAX_ATTEMPTS: usize = 3;
    tokio::time::sleep(Duration::from_millis(200)).await;
    for i in 1..=MAX_ATTEMPTS {
        match reqwest::get(url).await {
            Ok(_resp) => {
                bail!("[{label}] succeeded request when expecting to fail");
            }
            Err(_e) => {
                let sleep_for = 200u32 * 2u32.pow(i as u32);
                tokio::time::sleep(Duration::from_millis(u64::from(sleep_for))).await;
            }
        }
    }
    Ok(())
}
