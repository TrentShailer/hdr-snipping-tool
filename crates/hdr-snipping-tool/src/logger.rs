use log::LevelFilter;

use crate::project_directory;

pub fn init_fern() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(move |out, message, record| {
            let message = message.to_string();
            let time = chrono::Local::now().format("%F %r %:z").to_string();
            let level = record.level();
            let target = record.target();

            out.finish(format_args!("[{time}] [{level}] [{target}]\n{message}\n",))
        })
        .chain(
            fern::Dispatch::new()
                .level(LevelFilter::Warn)
                .chain(std::io::stdout())
                .chain(fern::log_file(project_directory().join("error.log"))?),
        )
        .apply()?;

    if std::env::var("hdr-snipping-tool-debug").is_ok() {
        fern::Dispatch::new()
            .format(move |out, message, _record| {
                let message = message.to_string();
                out.finish(format_args!("{message}"))
            })
            .filter(|metadata| metadata.level() == LevelFilter::Debug)
            .level(LevelFilter::Debug)
            .chain(std::io::stdout())
            .chain(fern::log_file(project_directory().join("debug.log"))?)
            .apply()?;
    }

    Ok(())
}
