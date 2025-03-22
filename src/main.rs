#[cfg(windows)]
extern "stdcall" {
    fn OutputDebugStringW(chars: *const u16);
    fn IsDebuggerPresent() -> i32;
}

fn output_debug_string(s: &str) {
    #[cfg(windows)]
    {
        let len = s.encode_utf16().count() + 1;
        let mut s_utf16: Vec<u16> = Vec::with_capacity(len + 1);
        s_utf16.extend(s.encode_utf16());
        s_utf16.push(0);
        unsafe {
            OutputDebugStringW(&s_utf16[0]);
        }
    }
}

fn main() {
    win_dbg_logger::DEBUGGER_LOGGER.set_force_log_without_debugger(true);
    let _ = log::set_logger(&win_dbg_logger::DEBUGGER_LOGGER);
    log::info!("Hello, world!");
}
