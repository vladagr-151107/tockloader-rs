use async_trait::async_trait;

use crate::connection::{Connection, TockloaderConnection};
use crate::errors::TockloaderError;
use crate::{CommandEraseApps, IO};

#[async_trait]
impl CommandEraseApps for TockloaderConnection {
    async fn erase_apps(&mut self) -> Result<(), TockloaderError> {
        self.write(self.get_settings().app_start_address, &[0u8])
            .await
    }
}
