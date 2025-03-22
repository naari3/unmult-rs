fn main() {
    win_dbg_logger::DEBUGGER_LOGGER.set_force_log_without_debugger(true);
    let _ = log::set_logger(&win_dbg_logger::DEBUGGER_LOGGER);
    log::info!("Hello, world!");
}
