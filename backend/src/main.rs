#![cfg_attr(all(windows, feature = "desktop"), windows_subsystem = "windows")]
mod application;
mod archive;
mod commands;
mod controller;
#[cfg(all(windows, feature = "desktop"))]
mod desktop;
mod dsp;
mod editing;
mod filesystem;
#[cfg(test)]
mod integration_tests;
mod lifecycle;
mod media;
mod mixer;
mod model;
mod native_files;
mod runtime;
mod server;
#[cfg(test)]
mod server_tests;
mod session;
mod state;
mod storage;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    application::run().await
}
