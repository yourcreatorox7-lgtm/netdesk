// پنجره‌ی کنسول را در ویندوز (نسخه‌ی release) پنهان می‌کند
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    netdesk_lib::run()
}
