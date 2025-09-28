use axum::http::HeaderMap;
use tokio::net::TcpListener;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut args = std::env::args();
    let port = args
        .nth(1)
        .expect("expected a single arg to the test runner, a port to run on");
    let port: u16 = port.parse().expect("failed to parse port");
    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&addr)
        .await
        .expect("failed to bind test server");
    let router = axum::Router::new()
        .route("/", axum::routing::get(index))
        .route("/big", axum::routing::get(big));
    axum::serve(listener, router).await.unwrap();
}

async fn index() -> (HeaderMap, &'static str) {
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "text/plain".parse().unwrap());
    (headers, "Hello, World!")
}

/// About a megabyte of payload to ensure the buffer functions
async fn big() -> (HeaderMap, String) {
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "text/plain".parse().unwrap());
    let mut content = String::with_capacity(2 * 1024 * 1024);
    for _i in 0..(1024 * 1024 / 8) {
        content.push_str("8-bytepl");
    }
    (headers, content)
}
