use chrono::Local;
use tracing::{
    level_filters::LevelFilter,
    subscriber::{set_global_default, SetGlobalDefaultError},
    Level,
};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt};

use crate::project_directory;

pub fn init_tracing(
    level: LevelFilter,
    log_span_close: bool,
) -> Result<[WorkerGuard; 2], SetGlobalDefaultError> {
    let filter = tracing_subscriber::filter::Targets::new()
        .with_default(level)
        .with_target("winit", LevelFilter::OFF);

    // file logger
    let file_name = file_name(level, log_span_close);
    let file_appender = tracing_appender::rolling::never(project_directory(), file_name);
    let (file_writer, _file_guard) = tracing_appender::non_blocking(file_appender);

    let file_logger = tracing_subscriber::fmt::layer()
        .with_writer(file_writer)
        .with_ansi(false)
        .with_target(false);

    let file_logger = if log_span_close {
        file_logger.with_span_events(FmtSpan::CLOSE)
    } else {
        file_logger
    };

    // stdout logger
    let (std_writer, _std_guard) = tracing_appender::non_blocking(std::io::stdout());
    let std_logger = tracing_subscriber::fmt::layer()
        .with_writer(std_writer)
        .with_ansi(false)
        .with_target(false);

    let std_logger = if log_span_close {
        std_logger.with_span_events(FmtSpan::CLOSE)
    } else {
        std_logger
    };

    // Register loggers
    let collector = tracing_subscriber::registry()
        .with(file_logger)
        .with(std_logger)
        .with(filter);

    set_global_default(collector)?;

    Ok([_file_guard, _std_guard])
}

fn file_name(level: LevelFilter, log_span_close: bool) -> String {
    if log_span_close {
        if let Some(level) = level.into_level() {
            if level == Level::DEBUG {
                let timestamp = Local::now().format("%F_%H-%M-%S");
                return format!("hdr-snipping-tool.timing.{}.log", timestamp);
            }
        }

        return "hdr-snipping-tool.timing.log".to_string();
    }

    "hdr-snipping-tool.log".to_string()
}

// fn register_verbose_logger() -> Result<WorkerGuard, SetGlobalDefaultError> {}

// fn register_timing_logger() -> Result<[WorkerGuard; 2], SetGlobalDefaultError> {
//     let filter = tracing_subscriber::filter::Targets::new()
//         .with_default(LevelFilter::DEBUG)
//         .with_target("winit", LevelFilter::OFF);

//     // let timing_filter = tracing_subscriber::filter::FilterFn::new(|metadata| {
//     //     metadata.level() == &LevelFilter::INFO
//     // })
//     // .with_max_level_hint(LevelFilter::INFO);

//     let file_appender =
//         tracing_appender::rolling::never(project_directory(), "hdr-snipping-tool.timing.log");
//     let (file_writer, _file_guard) = tracing_appender::non_blocking(file_appender);
//     let file_logger = tracing_subscriber::fmt::layer()
//         .with_writer(file_writer)
//         .with_span_events(FmtSpan::CLOSE)
//         .with_ansi(false)
//         .with_target(false);
//     // .with_filter(timing_filter.clone());

//     let (std_writer, _std_guard) = tracing_appender::non_blocking(std::io::stdout());
//     let std_logger = tracing_subscriber::fmt::layer()
//         .with_writer(std_writer)
//         .with_span_events(FmtSpan::CLOSE)
//         .with_ansi(false)
//         .with_target(false);

//     let collector = tracing_subscriber::registry()
//         .with(file_logger)
//         .with(std_logger)
//         .with(filter);

//     set_global_default(collector)?;

//     Ok([_file_guard, _std_guard])
// }

// fn register_debug_logger() -> Result<[WorkerGuard; 2], SetGlobalDefaultError> {
//     let filter = tracing_subscriber::filter::Targets::new()
//         .with_default(LevelFilter::DEBUG)
//         .with_target("winit", LevelFilter::OFF);

//     let file_appender =
//         tracing_appender::rolling::never(project_directory(), "hdr-snipping-tool.log");
//     let (file_writer, _file_guard) = tracing_appender::non_blocking(file_appender);
//     let file_logger = tracing_subscriber::fmt::layer()
//         .with_writer(file_writer)
//         .with_ansi(false)
//         .with_target(false);

//     let (std_writer, _std_guard) = tracing_appender::non_blocking(std::io::stdout());
//     let std_logger = tracing_subscriber::fmt::layer()
//         .with_writer(std_writer)
//         .with_ansi(false)
//         .with_target(false);

//     let collector = tracing_subscriber::registry()
//         .with(file_logger)
//         .with(std_logger)
//         .with(filter);

//     set_global_default(collector)?;

//     Ok([_file_guard, _std_guard])
// }

// fn register_error_logger() -> Result<[WorkerGuard; 2], SetGlobalDefaultError> {
//     let filter = tracing_subscriber::filter::Targets::new()
//         .with_default(LevelFilter::INFO)
//         .with_target("winit", LevelFilter::OFF);

//     let file_appender =
//         tracing_appender::rolling::never(project_directory(), "hdr-snipping-tool.log");
//     let (file_writer, _file_guard) = tracing_appender::non_blocking(file_appender);
//     let file_logger = tracing_subscriber::fmt::layer()
//         .with_writer(file_writer)
//         .with_ansi(false)
//         .with_target(false);

//     let (std_writer, _std_guard) = tracing_appender::non_blocking(std::io::stdout());
//     let std_logger = tracing_subscriber::fmt::layer()
//         .with_writer(std_writer)
//         .with_ansi(false)
//         .with_target(false);

//     let collector = tracing_subscriber::registry()
//         .with(file_logger)
//         .with(std_logger)
//         .with(filter);

//     set_global_default(collector)?;

//     Ok([_file_guard, _std_guard])
// }
