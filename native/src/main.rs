// SPDX-License-Identifier: GPL-3.0-or-later
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod chart;
mod i18n;
mod ui;

fn main() {
    #[cfg(windows)]
    // Keep double-click launches console-free while supporting pip CLI output.
    unsafe {
        windows_sys::Win32::System::Console::AttachConsole(u32::MAX);
    }
    if let Err(error) = ui::run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
