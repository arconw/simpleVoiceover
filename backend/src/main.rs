#![cfg_attr(all(windows, feature = "desktop"), windows_subsystem = "windows")]
mod application;
mod archive;
mod commands;
mod controller;
#[cfg(feature = "desktop")]
mod desktop;
mod dsp;
mod editing;
mod filesystem;
mod i18n;
mod import_reader;
mod indexed_archive;
#[cfg(test)]
mod integration_tests;
mod lifecycle;
mod loudness;
#[cfg(test)]
mod loudness_tests;
mod media;
mod mixer;
mod model;
mod native_files;
#[cfg(test)]
mod performance_tests;
#[cfg(test)]
mod preferences_tests;
mod progress;
mod regions;
#[cfg(test)]
mod regions_tests;
mod runtime;
mod server;
#[cfg(test)]
mod server_tests;
mod session;
mod state;
mod storage;
#[cfg(test)]
mod storage_tests;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    application::run().await
}
