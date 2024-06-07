pub fn init_fern() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(move |out, message, record| {
            let message = message.to_string();
            let time = chrono::Local::now().format("%F %r %:z").to_string();
            let level = record.level();
            let target = record.target();

            out.finish(format_args!("[{time}] [{level}] [{target}]\n{message}\n",))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file("log.txt")?)
        .apply()?;
    Ok(())
}
