use async_trait::async_trait;

use crate::attributes::app_attributes::AppAttributes;
use crate::attributes::system_attributes::SystemAttributes;
use crate::connection::TockloaderConnection;
use crate::errors::TockloaderError;
use crate::{IOCommands, IO};

#[async_trait]
impl IO for TockloaderConnection {
    async fn read(&mut self, address: u64, size: usize) -> Result<Vec<u8>, TockloaderError> {
        match self {
            TockloaderConnection::ProbeRS(conn) => conn.read(address, size).await,
            TockloaderConnection::Serial(conn) => conn.read(address, size).await,
        }
    }

    async fn write(&mut self, address: u64, pkt: &[u8]) -> Result<(), TockloaderError> {
        match self {
            TockloaderConnection::ProbeRS(conn) => conn.write(address, pkt).await,
            TockloaderConnection::Serial(conn) => conn.write(address, pkt).await,
        }
    }

    async fn erase_page(&mut self, address: u64) -> Result<(), TockloaderError> {
        match self {
            TockloaderConnection::ProbeRS(conn) => conn.erase_page(address).await,
            TockloaderConnection::Serial(conn) => conn.erase_page(address).await,
        }
    }
}

#[async_trait]
impl IOCommands for TockloaderConnection {
    async fn read_installed_apps(&mut self) -> Result<Vec<AppAttributes>, TockloaderError> {
        match self {
            TockloaderConnection::ProbeRS(conn) => conn.read_installed_apps().await,
            TockloaderConnection::Serial(conn) => conn.read_installed_apps().await,
        }
    }

    async fn read_system_attributes(&mut self) -> Result<SystemAttributes, TockloaderError> {
        match self {
            TockloaderConnection::ProbeRS(conn) => conn.read_system_attributes().await,
            TockloaderConnection::Serial(conn) => conn.read_system_attributes().await,
        }
    }
}
