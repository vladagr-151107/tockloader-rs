use async_trait::async_trait;

use crate::attributes::app_attributes::AppAttributes;
use crate::command_impl::reshuffle_apps::{create_pkt, reshuffle_apps, TockApp};
use crate::connection::{Connection, TockloaderConnection};
use crate::errors::{InternalError, TockloaderError};
use crate::{CommandEraseApps, CommandList, IO};

#[async_trait]
impl CommandEraseApps for TockloaderConnection {
    async fn erase_apps(&mut self) -> Result<(), TockloaderError> {
        let settings = self.get_settings();

        let app_attributes_list: Vec<AppAttributes> = self.list().await?;
        
        // if there are no apps on board, invalidate the first header
        if app_attributes_list.is_empty() {
            self.erase_page(settings.app_start_address).await?;
            return Ok(());
        }

        // split into apps we're keeping and erasing
        let (kept_attrs, removed_attrs): (Vec<&AppAttributes>, Vec<&AppAttributes>) =
            app_attributes_list
                .iter()
                .partition(|app| app.tbf_header.sticky());

        for app in &removed_attrs {
            log::info!("Erasing app at {:#x}.", app.address);
        }

        // shallow erase of non-sticky apps
        if kept_attrs.is_empty() {
            self.erase_page(settings.app_start_address).await?;
            log::info!("All apps have been erased.");
            return Ok(());
        }

        // read binaries for apps that are sticky
        let mut kept_binaries: Vec<Vec<u8>> = Vec::with_capacity(kept_attrs.len());
        for app in &kept_attrs {
            kept_binaries.push(
                self.read(app.address, app.tbf_header.total_size() as usize)
                    .await?,
            );
        }

        let kept_tock_apps: Vec<TockApp> = kept_attrs
            .iter()
            .map(|app| TockApp::from_app_attributes(app))
            .collect();

        let configuration =
            reshuffle_apps(&settings, kept_tock_apps).ok_or(TockloaderError::Internal(
                InternalError::MisconfiguredBoardSettings("Can't fit remaining apps".to_string()),
            ))?;

        let pkt = create_pkt(configuration, kept_binaries, None, &settings);

        // write reshuffled remaining apps back
        self.write(settings.app_start_address, &pkt).await?;

        // erase the page after them
        let tail_addr = settings.app_start_address + pkt.len() as u64;
        self.erase_page(tail_addr).await?;

        log::info!("After erasing apps, remaining apps on board:");
        for app in &kept_attrs {
            log::info!("  app at {:#x}", app.address);
        }

        Ok(())
    }
}