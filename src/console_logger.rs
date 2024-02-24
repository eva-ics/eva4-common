use once_cell::sync::Lazy;

static CONSOLE_LOG_NO_TIMESTAMP: Lazy<bool> =
    Lazy::new(|| std::env::var("EVA_CONSOLE_LOG_NO_TIMESTAMP").map_or(false, |v| v == "1"));

#[inline]
pub fn console_log_with_timestamp() -> bool {
    !*CONSOLE_LOG_NO_TIMESTAMP
}

pub fn configure_env_logger(verbose: bool) {
    let mut builder = env_logger::Builder::new();
    builder.target(env_logger::Target::Stdout);
    builder.filter_level(if verbose {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Info
    });
    if !console_log_with_timestamp() {
        builder.format_timestamp(None);
    }
    builder.init();
}
