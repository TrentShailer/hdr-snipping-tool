use log::LevelFilter;

use crate::project_directory;

pub fn init_fern() -> Result<(), fern::InitError> {
    let log_level = if std::env::var("hdr-snipping-tool-debug").is_ok() {
        LevelFilter::Debug
    } else {
        LevelFilter::Warn
    };

    fern::Dispatch::new()
        .format(move |out, message, record| {
            let message = message.to_string();
            let time = chrono::Local::now().format("%F %r %:z").to_string();
            let level = record.level();
            let target = record.target();

            out.finish(format_args!("[{time}] [{level}] [{target}]\n{message}\n",))
        })
        .level(LevelFilter::Warn)
        .level_for("hdr_snipping_tool", log_level)
        .level_for("vulkan_instance", log_level)
        .level_for("vulkan_renderer", log_level)
        .level_for("scrgb_tonemapper", log_level)
        .level_for("windows_capture_provider", log_level)
        .chain(std::io::stdout())
        .chain(fern::log_file(project_directory().join("log.txt"))?)
        .apply()?;
    Ok(())
}
