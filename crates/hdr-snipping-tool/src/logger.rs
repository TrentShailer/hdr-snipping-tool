use tracing::{
    level_filters::LevelFilter,
    subscriber::{set_global_default, SetGlobalDefaultError},
};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, Layer};

use crate::project_directory;

pub fn init_tracing() -> Result<(WorkerGuard, WorkerGuard), SetGlobalDefaultError> {
    let filter = tracing_subscriber::filter::Targets::new()
        .with_default(LevelFilter::TRACE)
        .with_target("winit", LevelFilter::OFF);

    // timing logger
    let timing_filter = tracing_subscriber::filter::FilterFn::new(|metadata| {
        metadata.level() == &LevelFilter::INFO
    })
    .with_max_level_hint(LevelFilter::INFO);
    let timing_file_appender =
        tracing_appender::rolling::never(project_directory(), "hdr-snipping-tool.timing.log");
    let (timing_non_blocking, _timing_guard) = tracing_appender::non_blocking(timing_file_appender);
    let timing_file_logger = tracing_subscriber::fmt::layer()
        .with_writer(timing_non_blocking)
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .with_target(false)
        .with_filter(timing_filter.clone());
    let timing_std_logger = tracing_subscriber::fmt::layer()
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .with_target(false)
        .with_filter(timing_filter);

    // error logger
    let file_appender =
        tracing_appender::rolling::never(project_directory(), "hdr-snipping-tool.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let error_file_logger = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(false);
    let error_std_logger = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_target(false);

    let collector = tracing_subscriber::registry()
        .with(error_file_logger)
        .with(error_std_logger)
        .with(timing_file_logger)
        .with(timing_std_logger)
        .with(filter);

    set_global_default(collector)?;

    Ok((_guard, _timing_guard))
}
