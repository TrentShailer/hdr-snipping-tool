use tracing::{
    level_filters::LevelFilter,
    subscriber::{set_global_default, SetGlobalDefaultError},
};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt};

use crate::{is_debug, project_directory};

pub fn init_tracing() -> Result<WorkerGuard, SetGlobalDefaultError> {
    let level = if is_debug() {
        LevelFilter::TRACE
    } else {
        LevelFilter::ERROR
    };

    let filter = tracing_subscriber::filter::Targets::new()
        .with_default(level)
        .with_target("winit", LevelFilter::OFF);

    let file_appender =
        tracing_appender::rolling::never(project_directory(), "hdr-snipping-tool.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let file_logger = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .with_target(false);

    let std_logger = tracing_subscriber::fmt::layer()
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .with_target(false);

    let collector = tracing_subscriber::registry()
        .with(file_logger)
        .with(std_logger)
        .with(filter);

    set_global_default(collector)?;

    Ok(_guard)
}
