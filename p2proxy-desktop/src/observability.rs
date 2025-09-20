use tracing::{Level, Metadata, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;

pub(super) fn setup_observability() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(CustomFilterWithBaseline(Level::DEBUG))
        .init();
}

struct CustomFilterWithBaseline(Level);

impl<S> Layer<S> for CustomFilterWithBaseline
where
    S: Subscriber,
{
    fn enabled(&self, metadata: &Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        let full = metadata.target();
        let (crate_toplevel, _) = full.split_once(':').unwrap_or((full, ""));
        match crate_toplevel {
            "winit" | "iced_wgpu" | "iced_winit" | "naga" | "cosmic_text" | "wgpu_core"
            | "wgpu_hal" | "hyper_util" | "rustls" | "iroh" | "iroh_quinn_proto" | "iroh_relay"
            | "portmapper" | "hickory_proto" | "hickory_resolver" | "igd_next" | "reqwest"
            | "netwatch" => {
                // Cmp is reversed here for some reason, ie warn is lower than INFO
                return metadata.level() < &Level::INFO;
            }
            _ => {}
        }
        match full {
            "events.net.relay.connected" => metadata.level() < &Level::INFO,
            _ => metadata.level() <= &self.0,
        }
    }
}
