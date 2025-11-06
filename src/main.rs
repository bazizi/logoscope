#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod parser;

mod utils;

use crate::utils::get_config_dir_path;

mod app;
use crate::app::LogoscopeApp;

mod view;

mod tab;

mod table;

mod messages;

mod theme;

pub fn main() -> iced::Result {
    env_logger::init();
    std::fs::create_dir_all(get_config_dir_path()).unwrap();
    iced::daemon(LogoscopeApp::new, LogoscopeApp::update, LogoscopeApp::view)
        .title(LogoscopeApp::title)
        .subscription(LogoscopeApp::subscription)
        .theme(LogoscopeApp::theme)
        .run()
}
