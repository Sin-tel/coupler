use log::info;
use log::LevelFilter;

pub fn init_logging() {
    #[cfg(debug_assertions)]
    simple_logging::log_to_file("out.log", LevelFilter::Info).ok();
    #[cfg(not(debug_assertions))]
    simple_logging::log_to_file("out.log", LevelFilter::Error).ok();

    log_panics::init();
    info!("Logging initialized");
}
