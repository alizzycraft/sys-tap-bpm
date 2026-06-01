#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    sys_tap_bpm_lib::run();
}
