use async_trait::async_trait;

use crate::attributes::general_attributes::GeneralAttributes;
use crate::connection::TockloaderConnection;
use crate::errors::TockloaderError;
use crate::{CommandInfo, IOCommands};

#[async_trait]
impl CommandInfo for TockloaderConnection {
    async fn info(&mut self) -> Result<GeneralAttributes, TockloaderError> {
        let installed_apps = self.read_installed_apps().await?;
        let system_atributes = self.read_system_attributes().await?;
        Ok(GeneralAttributes::new(system_atributes, installed_apps))
    }
}
