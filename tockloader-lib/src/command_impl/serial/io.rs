use async_trait::async_trait;

use crate::{
    attributes::{app_attributes::AppAttributes, system_attributes::SystemAttributes},
    bootloader_serial::{issue_command, ping_bootloader_and_wait_for_response, Command, Response},
    connection::{Connection, SerialConnection},
    errors::{InternalError, TockloaderError},
    IOCommands, IO,
};

#[async_trait]
impl IO for SerialConnection {
    async fn read(&mut self, address: u64, size: usize) -> Result<Vec<u8>, TockloaderError> {
        let mut pkt = (address as u32).to_le_bytes().to_vec();
        pkt.append(&mut (size as u16).to_le_bytes().to_vec());
        let stream = self.stream.as_mut().expect("Board must be open");

        let (_, appdata) = issue_command(
            stream,
            Command::ReadRange,
            pkt,
            true,
            size,
            Response::ReadRange,
        )
        .await?;

        if appdata.len() < size {
            // Sanity check that we wrote everything. This was previously failing
            // due to not reading enough when encountering double ESCAPE_CHAR.
            panic!("Internal Error: When reading from a Serial connection, we read less bytes than requested despite previous checks.");
        }
        Ok(appdata)
    }

    async fn write(&mut self, address: u64, pkt: &[u8]) -> Result<(), TockloaderError> {
        let page_size = self.get_settings().page_size as usize;
        let stream = self.stream.as_mut().expect("Board must be open");
        let mut binary = pkt.to_vec();

        if !binary.len().is_multiple_of(page_size) {
            binary.extend(vec![0u8; page_size - (binary.len() % page_size)]);
        }

        for page_number in 0..(binary.len() / page_size) {
            let mut pkt = (address as u32 + page_number as u32 * page_size as u32)
                .to_le_bytes()
                .to_vec();
            pkt.append(
                &mut binary[(page_number * page_size)..((page_number + 1) * page_size)].to_vec(),
            );
            let _ = issue_command(stream, Command::WritePage, pkt, true, 0, Response::OK).await?;
        }

        let pkt = (address as u32 + binary.len() as u32)
            .to_le_bytes()
            .to_vec();

        let _ = issue_command(stream, Command::ErasePage, pkt, true, 0, Response::OK).await?;
        Ok(())
    }

    async fn erase_page(&mut self, address: u64) -> Result<(), TockloaderError> {
        let stream = self.stream.as_mut().expect("Board must be open.");

        let pkt = (address as u32).to_le_bytes().to_vec();
        let _ = issue_command(stream, Command::ErasePage, pkt, true, 0, Response::OK).await?;
        Ok(())
    }
}

#[async_trait]
impl IOCommands for SerialConnection {
    async fn read_installed_apps(&mut self) -> Result<Vec<AppAttributes>, TockloaderError> {
        if !self.is_open() {
            return Err(InternalError::ConnectionNotOpen.into());
        }
        let stream = self.stream.as_mut().expect("Board must be open");

        ping_bootloader_and_wait_for_response(stream).await?;

        let start_address = self.get_settings().app_start_address;

        AppAttributes::read_apps_data(self, start_address).await
    }

    async fn read_system_attributes(&mut self) -> Result<SystemAttributes, TockloaderError> {
        if !self.is_open() {
            return Err(InternalError::ConnectionNotOpen.into());
        }
        let stream = self.stream.as_mut().expect("Board must be open");

        ping_bootloader_and_wait_for_response(stream).await?;

        let flash_address = self.get_settings().flash_start_address;
        let start_address = self.get_settings().app_start_address;
        let system_attributes =
            SystemAttributes::read_system_attributes(self, flash_address, start_address).await?;
        Ok(system_attributes)
    }
}
