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

        // if there are no apps detected on the board, ensure the start address is zeroed
        if app_attributes_list.is_empty() {
            self.write(settings.app_start_address, &[0u8; 8]).await?;
            return Ok(());
        }

        // calculate the highest address occupied by any app
        let total_apps_region_len: u64 = app_attributes_list
            .iter()
            .map(|app| {
                (app.address - settings.app_start_address) + app.tbf_header.total_size() as u64
            })
            .max()
            .unwrap_or(0);

        let mut app_binaries: Vec<Vec<u8>> = Vec::new();
        for app in app_attributes_list.iter() {
            app_binaries.push(
                self.read(app.address, app.tbf_header.total_size() as usize)
                    .await?,
            );
        }

        // filter out non-sticky apps
        let (kept_attrs, kept_binaries): (Vec<&AppAttributes>, Vec<Vec<u8>>) = app_attributes_list
            .iter()
            .zip(app_binaries)
            .filter(|(app, _bin)| {
                let sticky = app.tbf_header.sticky();
                if sticky {
                    log::info!(
                        "Not erasing app at {:#x} because it is sticky.",
                        app.address
                    );
                }
                sticky
            })
            .unzip();

        // zero out the entire area occupied previously, if all apps have been erased
        if kept_attrs.is_empty() {
            let zero_buf = vec![0u8; total_apps_region_len as usize];
            self.write(settings.app_start_address, &zero_buf).await?;
            log::info!("All apps have been erased.");
            return Ok(());
        }

        // used for sticky apps unable to erase
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

        // zero out all leftover trailing flash
        if (pkt.len() as u64) < total_apps_region_len {
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
