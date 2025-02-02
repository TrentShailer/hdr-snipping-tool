use tracing::{subscriber::set_global_default, Level};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;

use crate::config_dir;

pub fn setup_logger(should_debug: bool) -> [WorkerGuard; 2] {
    let level = if should_debug {
        Level::DEBUG
    } else {
        Level::INFO
    };

    let filter = tracing_subscriber::filter::Targets::new().with_default(level);

    // file logger
    let file_appender = tracing_appender::rolling::never(config_dir(), "hdr-snipping-tool.log");
    let (file_writer, _file_guard) = tracing_appender::non_blocking(file_appender);

    let file_logger = tracing_subscriber::fmt::layer()
        .with_writer(file_writer)
        .with_ansi(false)
        .with_target(false)
        .without_time();

    // stdout logger
    let (std_writer, _std_guard) = tracing_appender::non_blocking(std::io::stdout());
    let std_logger = tracing_subscriber::fmt::layer()
        .with_writer(std_writer)
        .with_ansi(false)
        .with_target(false)
        .without_time();

    // Register loggers
    let collector = tracing_subscriber::registry()
        .with(file_logger)
        .with(std_logger)
        .with(filter);

    set_global_default(collector).expect("Failed to set global logger");

    [_file_guard, _std_guard]
}
