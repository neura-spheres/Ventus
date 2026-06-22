#![cfg_attr(windows, windows_subsystem = "windows")]
#![deny(warnings)]
#![allow(dead_code)]

mod adblock;
mod ai;
mod app;
mod browser;
mod cloud;
mod config;
mod notify;
mod runtime;
mod storage;
mod ui;
mod updater;
mod utils;
mod version;

fn main() {
    runtime::run();
}
