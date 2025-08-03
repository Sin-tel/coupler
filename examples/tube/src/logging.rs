use log::error;
use log::LevelFilter;

pub fn init_logging() {
    simple_logging::log_to_file("out.log", LevelFilter::Info).ok();
    log_panics::init();

    error!("Hello world!");
}
