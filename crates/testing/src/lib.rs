//! # Testing
//! Library for resources used in the test binaries.
//!

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use tracing::{
    Level, info,
    subscriber::{SetGlobalDefaultError, set_global_default},
};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt};

pub fn setup_logger() -> Result<WorkerGuard, SetGlobalDefaultError> {
    let filter = tracing_subscriber::filter::Targets::new()
        .with_default(Level::TRACE)
        .with_target("winit", Level::WARN);

    // stdout logger
    let (std_writer, std_guard) = tracing_appender::non_blocking(std::io::stdout());
    let std_logger = tracing_subscriber::fmt::layer()
        .with_writer(std_writer)
        .with_ansi(false)
        .with_target(false)
        .with_span_events(FmtSpan::CLOSE | FmtSpan::ENTER);

    // Register loggers
    let collector = tracing_subscriber::registry().with(std_logger).with(filter);

    set_global_default(collector)?;

    info!("Application Start");
    Ok(std_guard)
}
