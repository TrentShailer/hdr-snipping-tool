use owo_colors::{OwoColorize, Style};

pub fn init_fern() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(move |out, message, record| {
            let message = message.to_string();
            let time = chrono::Local::now().format("%F %r %:z").to_string();
            let level = record.level();
            let target = record.target();
            let style = match level {
                log::Level::Error => Style::new().red(),
                log::Level::Warn => Style::new().yellow(),
                log::Level::Info => Style::new().blue(),
                log::Level::Debug => Style::new().cyan(),
                log::Level::Trace => Style::new().cyan(),
            };
            out.finish(format_args!(
                "[{time}] [{level}] [{target}]\n{message}\n",
                time = time.style(style),
                level = level.style(style),
                target = target.style(style),
                message = message,
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file("log.txt")?)
        .apply()?;
    Ok(())
}
