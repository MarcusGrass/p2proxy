use flutter_rust_bridge::{PanicBacktrace, frb};

#[frb(init)]
pub fn init_app() {
    setup_log_to_console();
    setup_backtrace();
}

fn setup_backtrace() {
    #[cfg(not(target_family = "wasm"))]
    if std::env::var("RUST_BACKTRACE").err() == Some(std::env::VarError::NotPresent) {
        unsafe {
            std::env::set_var("RUST_BACKTRACE", "1");
        }
    } else {
        log::debug!("Skip setup RUST_BACKTRACE because there is already environment variable");
    }

    PanicBacktrace::setup();
}

fn setup_log_to_console() {
    #[cfg(target_os = "android")]
    let filter = env_filter::Builder::new()
        // Defaults to no logging for unspecified modules without the None
        .filter(None, log::LevelFilter::Warn)
        .filter(Some("p2proxy_client"), log::LevelFilter::Debug)
        .build();
    #[cfg(target_os = "android")]
    let _ = android_logger::init_once(
        android_logger::Config::default()
            // Defaults to silent without this
            .with_max_level(log::LevelFilter::Debug)
            .with_filter(filter),
    );

    #[cfg(any(target_os = "ios", target_os = "macos"))]
    let _ = oslog::OsLogger::new("frb_user")
        .level_filter(log::LevelFilter::Trace)
        .init();
}
