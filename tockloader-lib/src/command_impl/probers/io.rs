use async_trait::async_trait;
use probe_rs::{flashing::DownloadOptions, MemoryInterface};

use crate::{
    attributes::{app_attributes::AppAttributes, system_attributes::SystemAttributes},
    connection::{Connection, ProbeRSConnection},
    errors::{InternalError, TockloaderError},
    IOCommands, IO,
};

#[async_trait]
impl IO for ProbeRSConnection {
    async fn read(&mut self, address: u64, size: usize) -> Result<Vec<u8>, TockloaderError> {
        if !self.is_open() {
            return Err(InternalError::ConnectionNotOpen.into());
        }
        let session = self.session.as_mut().expect("Board must be open");

        let mut core = session.core(self.target_info.core)?;
        let mut appdata = vec![0u8; size];
        core.read(address, &mut appdata)?;
        Ok(appdata)
    }

    async fn write(&mut self, address: u64, pkt: &[u8]) -> Result<(), TockloaderError> {
        if !self.is_open() {
            return Err(InternalError::ConnectionNotOpen.into());
        }
        let session = self.session.as_mut().expect("Board must be open");
        let mut loader = session.target().flash_loader();

        loader.add_data(address, pkt)?;

        let mut options = DownloadOptions::default();
        options.keep_unwritten_bytes = true;

        loader.commit(session, options)?;
        Ok(())
    }
}

#[async_trait]
impl IOCommands for ProbeRSConnection {
    async fn read_installed_apps(&mut self) -> Result<Vec<AppAttributes>, TockloaderError> {
        if !self.is_open() {
            return Err(InternalError::ConnectionNotOpen.into());
        }
        let start_address = self.get_settings().app_start_address;
        AppAttributes::read_apps_data(self, start_address).await
    }

    async fn read_system_attributes(&mut self) -> Result<SystemAttributes, TockloaderError> {
        if !self.is_open() {
            return Err(InternalError::ConnectionNotOpen.into());
        }
        let flash_address = self.get_settings().flash_start_address;
        let start_address = self.get_settings().app_start_address;
        let system_attributes =
            SystemAttributes::read_system_attributes(self, flash_address, start_address).await?;
        Ok(system_attributes)
    }
}
