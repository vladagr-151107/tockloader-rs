use async_trait::async_trait;

use crate::attributes::app_attributes::AppAttributes;
use crate::command_impl::reshuffle_apps::{create_pkt, reshuffle_apps, TockApp};
use crate::connection::{Connection, TockloaderConnection};
use crate::errors::{InternalError, TockloaderError};
use crate::tabs::tab::Tab;
use crate::{CommandInstall, CommandList, IO};

#[async_trait]
impl CommandInstall for TockloaderConnection {
    async fn install_app(&mut self, tab: Tab) -> Result<(), TockloaderError> {
        let settings = self.get_settings();
        let app_attributes_list: Vec<AppAttributes> = self.list().await?;
        let mut tock_app_list = app_attributes_list
            .iter()
            .map(TockApp::from_app_attributes)
            .collect::<Vec<TockApp>>();
        log::info!("tock apps len {:?}", tock_app_list.len());

        // obtain the binaries in a vector
        let mut app_binaries: Vec<Vec<u8>> = Vec::new();

        let mut address = settings.app_start_address;
        for app in app_attributes_list.iter() {
            app_binaries.push(
                self.read(address, app.tbf_header.total_size() as usize)
                    .await
                    .unwrap(),
            );
            address += app.tbf_header.total_size() as u64;
        }

        let app = TockApp::from_tab(&tab, &settings).unwrap();

        tock_app_list.push(app.clone());

        let configuration =
            reshuffle_apps(&settings, tock_app_list).ok_or(TockloaderError::Internal(
                InternalError::MisconfiguredBoardSettings("Can't fit new app".to_string()),
            ))?;

        // create the pkt, this contains all the binaries in a vec
        let pkt = create_pkt(configuration, app_binaries, Some(tab), &settings);

        log::debug!("pkt len {}", pkt.len());
        // write the pkt
        self.write(settings.app_start_address, &pkt).await?;

        // erase exactly one page right after the just written apps
        let tail_addr = settings.app_start_address + pkt.len() as u64;
        self.erase_page(tail_addr).await?;

        Ok(())
    }
}
