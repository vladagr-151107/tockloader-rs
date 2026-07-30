#[derive(Clone)]
pub struct BoardSettings {
    pub arch: Option<String>,
    /// Previously named 'flash_address' in the python version of tockloader
    pub flash_start_address: u64,
    /// Previously named `start_address` in the python version of tockloader
    pub app_start_address: u64,
    pub page_size: u64,
    pub ram_start_address: u64,
}

// TODO(eva-cosma): Does a default implementation make sense for this? Is a
// 'None' architechture a sane idea?
impl Default for BoardSettings {
    fn default() -> Self {
        Self {
            arch: None,
            flash_start_address: 0x00000,
            app_start_address: 0x40000,
            page_size: 512,
            ram_start_address: 0x20000000,
        }
    }
}
