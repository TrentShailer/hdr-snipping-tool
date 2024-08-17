use tracing::subscriber::set_global_default;
use tracing_subscriber::{filter::LevelFilter, fmt::format::FmtSpan, layer::SubscriberExt};

pub fn init_logger() {
    let filter = tracing_subscriber::filter::Targets::new()
        .with_default(LevelFilter::TRACE)
        .with_target("winit", LevelFilter::OFF);

    let std_logger = tracing_subscriber::fmt::layer()
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .with_target(false)
        .without_time();

    let collector = tracing_subscriber::registry().with(std_logger).with(filter);

    set_global_default(collector).unwrap();
}
