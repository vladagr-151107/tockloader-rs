use async_trait::async_trait;

use crate::attributes::app_attributes::AppAttributes;
use crate::command_impl::reshuffle_apps::{create_pkt, reshuffle_apps, TockApp};
use crate::connection::{Connection, TockloaderConnection};
use crate::errors::{InternalError, TockloaderError};
use crate::{CommandEraseApps, CommandList, IO};

#[async_trait]
impl CommandEraseApps for TockloaderConnection {
    async fn erase_apps(&mut self, shallow: bool) -> Result<(), TockloaderError> {
        let settings = self.get_settings();

        let app_attributes_list: Vec<AppAttributes> = self.list().await?;

        // no apps on the board, nothing for us to do
        if app_attributes_list.is_empty() {
            return Ok(());
        }

        // for the full-erase path to know how much flash to zero
        let total_apps_region_len: u64 = app_attributes_list
            .iter()
            .map(|app| {
                (app.address - settings.app_start_address) + app.tbf_header.total_size() as u64
            })
            .max()
            .unwrap_or(0);

        // split into apps we're keeping and erasing
        let (kept_attrs, removed_attrs): (Vec<&AppAttributes>, Vec<&AppAttributes>) =
            app_attributes_list
                .iter()
                .partition(|app| app.tbf_header.sticky());

        for app in &removed_attrs {
            log::info!("Erasing app at {:#x}.", app.address);
        }

        // no sticky apps to keep
        if kept_attrs.is_empty() {
            if shallow {
                // shallow: only invalidate the first header, like upstream tockloader
                self.erase_page(settings.app_start_address).await?;
            } else {
                // full: physically zero out the entire region every app used to occupy
                let zero_buf = vec![0u8; total_apps_region_len as usize];
                self.write(settings.app_start_address, &zero_buf).await?;
            }
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

        if shallow {
            // shallow: erase just the page right after them
            let tail_addr = settings.app_start_address + pkt.len() as u64;
            self.erase_page(tail_addr).await?;
        } else if (pkt.len() as u64) < total_apps_region_len {
            // full: zero out everything from the new end of the list to where the apps used to end
            let tail_addr = settings.app_start_address + pkt.len() as u64;
            let tail_len = total_apps_region_len - pkt.len() as u64;
            let zero_tail = vec![0u8; tail_len as usize];
            self.write(tail_addr, &zero_tail).await?;
        }

        log::info!("After erasing apps, remaining apps on board:");
        for app in &kept_attrs {
            log::info!("  app at {:#x}", app.address);
        }

        Ok(())
    }
}
