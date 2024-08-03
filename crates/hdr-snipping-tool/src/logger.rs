use log::LevelFilter;

use crate::{is_debug, project_directory};

pub fn init_fern() -> Result<(), fern::InitError> {
    if is_debug() {
        init_debug_logger()?;
    } else {
        init_default_logger()?;
    };

    Ok(())
}

fn error_logger() -> fern::Dispatch {
    fern::Dispatch::new()
        .format(move |out, message, record| {
            let time = chrono::Local::now().format("%F %r %:z");
            let level = record.level();
            let target = record.target();

            out.finish(format_args!("[{time}] [{level}] [{target}]\n{message}\n",))
        })
        .level(LevelFilter::Warn)
}

fn debug_logger() -> fern::Dispatch {
    fern::Dispatch::new()
        .format(move |out, message, _record| out.finish(format_args!("{message}")))
        .level(LevelFilter::Debug)
}

fn init_default_logger() -> Result<(), fern::InitError> {
    error_logger()
        .chain(std::io::stdout())
        .chain(fern::log_file(project_directory().join("error.log"))?)
        .apply()?;

    Ok(())
}

fn init_debug_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .chain(error_logger().chain(fern::log_file(project_directory().join("error.log"))?))
        .chain(
            debug_logger()
                .chain(std::io::stdout())
                .chain(fern::log_file(project_directory().join("debug.log"))?),
        )
        .apply()?;

    Ok(())
}
