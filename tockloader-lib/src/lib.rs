// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OXIDOS AUTOMOTIVE 2024.

pub mod attributes;
pub mod board_settings;
pub(crate) mod bootloader_serial;
pub mod command_impl;
pub mod connection;
mod errors;
pub mod known_boards;
pub mod tabs;

use async_trait::async_trait;
use probe_rs::probe::DebugProbeInfo;
use tokio_serial::SerialPortInfo;

use crate::attributes::app_attributes::AppAttributes;
use crate::attributes::general_attributes::GeneralAttributes;
use crate::attributes::system_attributes::SystemAttributes;
use crate::errors::*;
use crate::tabs::tab::Tab;

pub fn list_debug_probes() -> Vec<DebugProbeInfo> {
    probe_rs::probe::list::Lister::new().list_all()
}

pub fn list_serial_ports() -> Result<Vec<SerialPortInfo>, TockloaderError> {
    tokio_serial::available_ports().map_err(|e| TockloaderError::Serial(e.into()))
}

// TODO(eva-cosma): Examine if we need to split these functions into smaller
// parts (reading - processing - writing) for mocking. Could also involve adding
// functions to the proposed 'Connection' trait.

// TODO(eva-cosma): General housekeeping in these functions.

#[async_trait]
pub trait CommandList {
    async fn list(&mut self) -> Result<Vec<AppAttributes>, TockloaderError>;
}

#[async_trait]
pub trait CommandInfo {
    async fn info(&mut self) -> Result<GeneralAttributes, TockloaderError>;
}

#[async_trait]
pub trait CommandInstall {
    async fn install_app(&mut self, tab_file: Tab) -> Result<(), TockloaderError>;
}

#[async_trait]
pub trait CommandEraseApps {
    async fn erase_apps(&mut self, shallow: bool) -> Result<(), TockloaderError>;
}

#[async_trait]
pub trait IO: Send {
    async fn read(&mut self, address: u64, size: usize) -> Result<Vec<u8>, TockloaderError>;

    async fn write(&mut self, address: u64, pkt: &[u8]) -> Result<(), TockloaderError>;

    async fn erase_page(&mut self, address: u64) -> Result<(), TockloaderError>;
}

#[async_trait]
pub trait IOCommands: Send {
    async fn read_installed_apps(&mut self) -> Result<Vec<AppAttributes>, TockloaderError>;

    async fn read_system_attributes(&mut self) -> Result<SystemAttributes, TockloaderError>;
}
