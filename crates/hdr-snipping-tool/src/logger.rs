use debug_helper::{is_debug, is_verbose};
use log::LevelFilter;

use crate::project_directory;

pub fn init_fern() -> Result<(), fern::InitError> {
    if is_verbose() {
        init_verbose_logger()?;
    } else if is_debug() {
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
        .level_for("scrgb_tonemapper::tonemap", LevelFilter::Off)
        .level_for("vulkan_renderer::renderer::render", LevelFilter::Off)
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

fn init_verbose_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .chain(error_logger().chain(fern::log_file(project_directory().join("error.log"))?))
        .chain(debug_logger().chain(fern::log_file(project_directory().join("debug.log"))?))
        .chain(
            fern::Dispatch::new()
                .format(move |out, message, _record| out.finish(format_args!("{message}")))
                .level(LevelFilter::Info)
                .level_for("scrgb_tonemapper::tonemap", LevelFilter::Debug)
                .chain(fern::log_file(
                    project_directory().join("_tonemap.debug.log"),
                )?),
        )
        .chain(
            fern::Dispatch::new()
                .format(move |out, message, _record| out.finish(format_args!("{message}")))
                .level(LevelFilter::Info)
                .level_for("vulkan_renderer::renderer::render", LevelFilter::Debug)
                .chain(fern::log_file(
                    project_directory().join("_render.debug.log"),
                )?),
        )
        .chain(
            fern::Dispatch::new()
                .format(move |out, message, _record| out.finish(format_args!("{message}")))
                .level(LevelFilter::Debug)
                .chain(std::io::stdout()),
        )
        .apply()?;

    Ok(())
}
