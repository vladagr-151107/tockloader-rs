use itertools::Itertools;
use log::warn;

use crate::attributes::app_attributes::AppAttributes;
use crate::board_settings::BoardSettings;
use crate::tabs::tab::Tab;
use tbf_parser::parse::{parse_tbf_header, parse_tbf_header_lengths};

const ALIGNMENT: u64 = 1024;

#[derive(Clone, Debug)]
pub enum TockApp {
    Flexible(FlexibleApp),
    Fixed(FixedApp),
}

#[derive(Clone, Debug)]
pub struct FlexibleApp {
    pub installed: bool,
    pub idx: Option<usize>,
    pub size: u64,
}

#[derive(Clone, Debug)]
pub struct FixedApp {
    pub installed: bool,
    pub idx: Option<usize>,
    // flash: u64 and ram: u64
    pub compatible_addresses: Vec<Option<(u64, u64)>>,
    pub size: u64,
}

impl TockApp {
    pub fn replace_idx(&mut self, new_idx: usize) -> Option<usize> {
        match self {
            TockApp::Flexible(flexible_app) => flexible_app.idx.replace(new_idx),
            TockApp::Fixed(fixed_app) => fixed_app.idx.replace(new_idx),
        }
    }

    pub fn from_app_attributes(app_attributes: &AppAttributes) -> TockApp {
        if let (Some(flash_addr), Some(ram_addr)) = (
            app_attributes.tbf_header.get_fixed_address_flash(),
            app_attributes.tbf_header.get_fixed_address_ram(),
        ) {
            let mut aligned_adr = align_down(flash_addr as u64);
            if aligned_adr < 0x00040000 {
                aligned_adr = 0x00040000;
            }
            // let flash_header = flash_addr + app_attributes.tbf_header.total_size() as u32;
            log::info!(
                "found fixed flash 0x{:08x} and fixed ram 0x{:08x}",
                aligned_adr,
                ram_addr
            );
            // panic!();
            // cortex-m4.0x00040000.0x20008000
            // cortex-m4.0x00040000.0x20008000
            log::info!("setting flash address form header {:#x}", aligned_adr);
            TockApp::Fixed(FixedApp {
                installed: true,
                idx: None,
                compatible_addresses: vec![(Some((aligned_adr, ram_addr as u64)))],
                size: app_attributes.tbf_header.total_size() as u64,
            })
        } else {
            TockApp::Flexible(FlexibleApp {
                installed: true,
                idx: None,
                size: app_attributes.tbf_header.total_size() as u64,
            })
        }
    }

    /// This function returns an instance of TockApp
    pub fn from_tab(tab: &Tab, settings: &BoardSettings) -> Option<TockApp> {
        // extract the binary
        // this should be changed to accomodate candidate_addresses
        let binary = tab
            .extract_binary(settings.arch.clone().unwrap())
            .expect("invalid arch");

        // extract relevant data from the header
        let (tbf_version, header_len, total_size) = match parse_tbf_header_lengths(
            &binary[0..8]
                .try_into()
                .expect("Buffer length must be at least 8 bytes long."),
        ) {
            Ok((tbf_version, header_len, total_size)) if header_len != 0 => {
                (tbf_version, header_len, total_size)
            }
            _ => return None,
        };

        // turn the buffer slice into a TbfHeader instance
        let header =
            parse_tbf_header(&binary[0..header_len as usize], tbf_version).expect("invalid header");

        if header.get_fixed_address_flash().is_some() {
            // addr = align_down(addr as u64) as u32;
            // if addr < settings.app_start_address as u32 {
            //     // this rust app should not be here
            //     panic!(
            //         "This rust app starts at {addr:#x}, while the board's app_start_address is {:#x}",
            //         settings.app_start_address
            //     )
            // }
            // turns out that fixed address is a loosely-used term, address has to be aligned down to a multiple of 1024 bytes
            // let address = align_down(addr as u64);

            Some(TockApp::Fixed(FixedApp {
                installed: false,
                idx: None,
                compatible_addresses: tab.filter_tbfs(settings).unwrap(), // (adi): change this when tbf selector gets merged
                size: total_size as u64,
            }))
        } else {
            Some(TockApp::Flexible(FlexibleApp {
                installed: false,
                idx: None,
                size: total_size as u64,
            }))
        }
    }
}

impl FixedApp {
    fn as_index(&self, ram_address: Option<u64>, install_address: u64) -> Index {
        Index {
            installed: self.installed,
            idx: self.idx,
            ram_address,
            address: install_address,
            size: self.size,
        }
    }
}

impl FlexibleApp {
    fn as_index(&self, ram_address: Option<u64>, install_address: u64) -> Index {
        Index {
            installed: self.installed,
            idx: self.idx,
            ram_address,
            address: install_address,
            size: self.size,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Index {
    pub installed: bool,
    pub idx: Option<usize>,
    pub ram_address: Option<u64>,
    pub address: u64,
    pub size: u64,
}

pub fn reshuffle_apps(
    settings: &BoardSettings,
    mut installed_apps: Vec<TockApp>,
) -> Option<Vec<Index>> {
    // On the first pass, we must assign every app its original index, so we can
    // keep track of it.
    for (idx, app) in installed_apps.iter_mut().enumerate() {
        if app.replace_idx(idx).is_some() {
            warn!("Encountered existing index in TockApp at the start of reorder_apps.");
        }
    }
    log::debug!("installed apps? {:#x?}", installed_apps);

    let mut rust_apps: Vec<&mut FixedApp> = Vec::new();
    let mut c_apps: Vec<&mut FlexibleApp> = Vec::new();

    for app in &mut installed_apps {
        match app {
            TockApp::Flexible(flexible_app) => c_apps.push(flexible_app),
            TockApp::Fixed(fixed_app) => rust_apps.push(fixed_app),
        }
    }

    // This places rust apps that were not installed in the front, so the candidate
    // addresses can be used. Otherwise, the algorithm will just look for the next
    // available address, leaving a lot of space in the front. This can happen
    // after uninstalling a rust app

    for app in &rust_apps {
        print!("rust app address(es): ");
        for address in &app.compatible_addresses {
            if address.is_some() {
                print!("[{:#x}, {:#x}]", address.unwrap().0, address.unwrap().1);
            }
        }
        println!();
    }

    rust_apps.sort_by_key(|app| app.compatible_addresses[0].unwrap().0);

    let mut initial_configuration: Vec<Index> = Vec::new();
    let mut new_rust_app: Option<&FixedApp> = None;
    for app in &rust_apps {
        if app.installed {
            initial_configuration.push(app.as_index(
                Some(app.compatible_addresses[0].unwrap().1),
                app.compatible_addresses[0].unwrap().0,
            ));
        } else {
            new_rust_app = Some(app);
        }
    }

    log::info!("first intial configuration: ");
    for app in &initial_configuration {
        log::info!(
            "app idx {}, addr {:#x}, size {}",
            app.idx.unwrap(),
            app.address,
            app.size
        );
    }

    if let Some(new_app) = new_rust_app {
        let mut found: bool = false;
        for candidate in new_app.compatible_addresses.iter().flatten() {
            let (flash_addr, ram_addr) = *candidate;
            let new_start = flash_addr;
            let new_end = flash_addr + new_app.size;
            log::info!(
                "checking for new_start {:#x}, new_end {:#x}",
                new_start,
                new_end
            );

            // Check before first element
            if initial_configuration.is_empty() {
                initial_configuration.push(new_app.as_index(Some(ram_addr), flash_addr));
                found = true;
                break;
            }

            // Check gap before first installed app
            let first = &initial_configuration[0];
            if new_end <= first.address {
                initial_configuration.insert(0, new_app.as_index(Some(ram_addr), flash_addr));
                found = true;
                break;
            }

            // Check gaps between apps
            for i in 0..initial_configuration.len() - 1 {
                let current = &initial_configuration[i];
                let next = &initial_configuration[i + 1];

                let gap_start = current.address + current.size;
                let gap_end = next.address;

                if new_start >= gap_start && new_end <= gap_end {
                    initial_configuration
                        .insert(i + 1, new_app.as_index(Some(ram_addr), flash_addr));
                    found = true;
                    break;
                }
            }

            // Check after last app
            let last = initial_configuration.last().unwrap();
            let gap_start = last.address + last.size;

            log::info!(
                "checking for last gap {:#x}, obtained with addr {:#x}, size {}",
                gap_start,
                last.address,
                last.size
            );

            if new_start >= gap_start {
                initial_configuration.push(new_app.as_index(Some(ram_addr), flash_addr));
                found = true;
                break;
            }
        }

        if !found {
            warn!("No suitable space found for new app");
            return None;
        }
    }

    println!("final intial configuration: ");
    for app in &initial_configuration {
        println!(
            "app idx {}, addr {:#x}, size {}",
            app.idx.unwrap(),
            app.address,
            app.size
        );
    }

    log::info!("sorted rust apps {:#x?}", rust_apps);
    // panic!();

    for app in &mut rust_apps {
        if app.compatible_addresses.is_empty() {
            warn!("Can not reorder apps since fixed application has no candidate addresses!");
            return None;
        }
    }

    // make permutations only for the c apps, as their order can be changed
    let mut permutations = (0..c_apps.len()).permutations(c_apps.len());

    let mut min_padding = usize::MAX;
    let mut saved_configuration: Vec<Index> = Vec::new();

    for _ in 0..100_000 {
        // use just 100k permutations, or else we'll be here for a while
        if let Some(order) = permutations.next() {
            let mut total_padding: usize = 0;
            let mut reordered_apps: Vec<Index> = initial_configuration.clone();

            for index in order {
                let c_app = &c_apps[index];
                let insert_size = c_app.size;

                if reordered_apps.is_empty() {
                    reordered_apps.push(c_app.as_index(None, settings.app_start_address));

                    continue;
                }

                log::info!("checking for intermediate");
                let mut inserted = false;

                for i in 0..reordered_apps.len() - 1 {
                    let base = reordered_apps[i].address + reordered_apps[i].size;
                    let gap = reordered_apps[i + 1].address - base;

                    let needed_padding = if !base.is_multiple_of(settings.page_size) {
                        settings.page_size - base % settings.page_size
                    } else {
                        0
                    };

                    let final_addr = base + needed_padding;

                    if insert_size + needed_padding <= gap {
                        if needed_padding > 0 {
                            total_padding += needed_padding as usize;

                            reordered_apps.insert(
                                i + 1,
                                Index {
                                    installed: false,
                                    idx: None,
                                    ram_address: None,
                                    address: base,
                                    size: needed_padding,
                                },
                            );

                            reordered_apps.insert(i + 2, c_app.as_index(None, final_addr));
                        } else {
                            reordered_apps.insert(i + 1, c_app.as_index(None, final_addr));
                        }

                        inserted = true;
                        break;
                    }
                }

                if !inserted {
                    let last = reordered_apps.last().unwrap();
                    let gap_start = last.address + last.size;

                    log::info!(
                        "checking for last gap {:#x}, obtained with addr {:#x}, size {}",
                        gap_start,
                        last.address,
                        last.size
                    );

                    let needed_padding = if !gap_start.is_multiple_of(settings.page_size) {
                        // c app needs to be inserted at a multiple of page_size
                        settings.page_size - gap_start % settings.page_size
                    } else {
                        0
                    };

                    if needed_padding > 0 {
                        // insert a padding
                        reordered_apps.push(Index {
                            installed: false,
                            idx: None,
                            ram_address: None,
                            address: gap_start,
                            size: needed_padding,
                        });
                        reordered_apps.push(c_app.as_index(None, gap_start + needed_padding));
                        total_padding += needed_padding as usize;
                    } else {
                        reordered_apps.push(c_app.as_index(None, gap_start));
                    }
                }
            }

            log::info!(
                "obtained config before padding check {:#x?}",
                reordered_apps
            );

            let mut i = 0;
            while i < reordered_apps.len() - 1 {
                let gap = reordered_apps[i + 1].address
                    - (reordered_apps[i].address + reordered_apps[i].size);

                if gap > 0 {
                    reordered_apps.insert(
                        i + 1,
                        Index {
                            installed: false,
                            idx: None,
                            ram_address: None,
                            address: reordered_apps[i].address + reordered_apps[i].size,
                            size: gap,
                        },
                    );
                    i += 1; // skip over inserted padding
                }

                i += 1;
            }

            // find the configuration that uses the minimum padding
            if total_padding < min_padding {
                min_padding = total_padding;
                saved_configuration = reordered_apps.clone();
                if min_padding == 0 {
                    break;
                }
            }
        } else {
            break;
        }
    }

    if let Some(last) = saved_configuration.last() {
        let end = last.address + last.size;
        if !end.is_multiple_of(settings.page_size) {
            let needed_padding = settings.page_size - end % settings.page_size;
            saved_configuration.push(Index {
                installed: false,
                idx: None,
                ram_address: None,
                address: end,
                size: needed_padding,
            });
        }
    }

    log::info!("obtained config {:#x?}", saved_configuration);
    // panic!();
    Some(saved_configuration)
}

/// This function returns the binary for a padding
fn create_padding(size: u32) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(&u16::to_le_bytes(2u16)); // tbf version 2
    buf.extend_from_slice(&u16::to_le_bytes(16u16)); // header size is 16
    buf.extend_from_slice(&u32::to_le_bytes(size)); // total_size is size
    let mut checksum = 0;
    for chunk in buf.as_chunks::<4>().0 {
        let word = u32::from_le_bytes(*chunk);
        checksum ^= word;
    }
    buf.extend_from_slice(&u32::to_le_bytes(checksum));
    while buf.len() < size as usize {
        buf.push(0x0u8);
    }
    buf
}

/// This function takes a rust app's fixed address and aligns it down to ALIGNMENT (1024 currently)
fn align_down(address: u64) -> u64 {
    address - address % ALIGNMENT
}

/// This function creates the full binary that will be written
pub fn create_pkt(
    configuration: Vec<Index>,
    mut app_binaries: Vec<Vec<u8>>,
    tab: Option<Tab>,
    settings: &BoardSettings,
) -> Vec<u8> {
    let mut pkt: Vec<u8> = Vec::new();
    for item in configuration.iter() {
        if let Some(idx) = item.idx {
            match &item.installed {
                true => pkt.append(&mut app_binaries[idx]),
                false => {
                    let mut arch: String = settings.arch.clone().unwrap();
                    // if ram is set, this is a rust app, we need to reconstruct the arch and then
                    // read the binary from the tab
                    if let Some(ram_address) = item.ram_address {
                        arch = format!("{}.0x{:08x}.0x{:08x}", arch, item.address, ram_address);
                    }
                    pkt.append(&mut tab.as_ref().unwrap().extract_binary(arch).unwrap());
                }
            }
        } else {
            // write padding binary
            let mut buf = create_padding(item.size as u32);
            pkt.append(&mut buf);
        }
    }
    pkt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_new_c_app() {
        let settings: &BoardSettings = &BoardSettings {
            arch: Some("cortex-m4".to_string()),
            flash_start_address: 0x00000000,
            app_start_address: 0x00040000,
            page_size: 512,
            ram_start_address: 0x20000000,
        };

        let c_app_1: TockApp = TockApp::Flexible(FlexibleApp {
            installed: true,
            idx: None,
            size: 0x2000,
        });

        let c_app_2: TockApp = TockApp::Flexible(FlexibleApp {
            installed: true,
            idx: None,
            size: 0x2000,
        });

        let c_app_3: TockApp = TockApp::Flexible(FlexibleApp {
            installed: false,
            idx: None,
            size: 0x2000,
        });

        let apps: Vec<TockApp> = vec![c_app_1, c_app_2, c_app_3];

        let reshuffled_apps = reshuffle_apps(settings, apps);

        let correct_config: Option<Vec<Index>> = Some(vec![
            Index {
                installed: true,
                idx: Some(0x0),
                ram_address: None,
                address: 0x40000,
                size: 0x2000,
            },
            Index {
                installed: true,
                idx: Some(0x1),
                ram_address: None,
                address: 0x42000,
                size: 0x2000,
            },
            Index {
                installed: false,
                idx: Some(0x2),
                ram_address: None,
                address: 0x44000,
                size: 0x2000,
            },
        ]);
        assert_eq!(reshuffled_apps, correct_config);
    }

    #[test]
    fn install_new_rust_app() {
        let settings: &BoardSettings = &BoardSettings {
            arch: Some("cortex-m4".to_string()),
            flash_start_address: 0x00000000,
            app_start_address: 0x00040000,
            page_size: 512,
            ram_start_address: 0x20000000,
        };

        let rust_app: TockApp = TockApp::Fixed(FixedApp {
            installed: false,
            idx: None,
            compatible_addresses: vec![
                Some((0x40000, 0x20008000)),
                Some((0x42000, 0x2000a000)),
                Some((0x48000, 0x20010000)),
                Some((0x80000, 0x20006000)),
                Some((0x88000, 0x2000e000)),
            ],
            size: 0x2000,
        });

        let apps: Vec<TockApp> = vec![rust_app];

        let reshuffled_apps = reshuffle_apps(settings, apps);

        let correct_config: Option<Vec<Index>> = Some(vec![Index {
            installed: false,
            idx: Some(0x0),
            ram_address: Some(0x20008000),
            address: 0x40000,
            size: 0x2000,
        }]);
        assert_eq!(reshuffled_apps, correct_config);
    }

    #[test]
    fn install_more_rust_apps() {
        let settings: &BoardSettings = &BoardSettings {
            arch: Some("cortex-m4".to_string()),
            flash_start_address: 0x00000000,
            app_start_address: 0x00040000,
            page_size: 512,
            ram_start_address: 0x20000000,
        };

        let rust_app_1: TockApp = TockApp::Fixed(FixedApp {
            installed: true,
            idx: None,
            compatible_addresses: vec![Some((0x40000, 0x20008000))],
            size: 0x2000,
        });

        let rust_app_2: TockApp = TockApp::Fixed(FixedApp {
            installed: true,
            idx: None,
            compatible_addresses: vec![Some((0x42000, 0x2000a000))],
            size: 0x2000,
        });

        let rust_app_3: TockApp = TockApp::Fixed(FixedApp {
            installed: false,
            idx: None,
            compatible_addresses: vec![
                Some((0x40000, 0x20008000)),
                Some((0x42000, 0x2000a000)),
                Some((0x48000, 0x20010000)),
                Some((0x80000, 0x20006000)),
                Some((0x88000, 0x2000e000)),
            ],
            size: 0x2000,
        });

        let apps: Vec<TockApp> = vec![rust_app_1, rust_app_2, rust_app_3];

        let reshuffled_apps = reshuffle_apps(settings, apps);

        let correct_config: Option<Vec<Index>> = Some(vec![
            Index {
                installed: true,
                idx: Some(0x0),
                ram_address: Some(0x20008000),
                address: 0x40000,
                size: 0x2000,
            },
            Index {
                installed: true,
                idx: Some(0x1),
                ram_address: Some(0x2000a000),
                address: 0x42000,
                size: 0x2000,
            },
            // padding
            Index {
                installed: false,
                idx: None,
                ram_address: None,
                address: 0x44000,
                size: 0x4000,
            },
            Index {
                installed: false,
                idx: Some(0x2),
                ram_address: Some(0x20010000),
                address: 0x48000,
                size: 0x2000,
            },
        ]);
        assert_eq!(reshuffled_apps, correct_config);
    }

    #[test]
    fn insert_c_app_between_rust_apps() {
        let settings: &BoardSettings = &BoardSettings {
            arch: Some("cortex-m4".to_string()),
            flash_start_address: 0x00000000,
            app_start_address: 0x00040000,
            page_size: 512,
            ram_start_address: 0x20000000,
        };

        let rust_app_1: TockApp = TockApp::Fixed(FixedApp {
            installed: true,
            idx: None,
            compatible_addresses: vec![Some((0x40000, 0x20008000))],
            size: 0x2000,
        });

        let rust_app_2: TockApp = TockApp::Fixed(FixedApp {
            installed: true,
            idx: None,
            compatible_addresses: vec![Some((0x42000, 0x2000a000))],
            size: 0x2000,
        });

        let rust_app_3: TockApp = TockApp::Fixed(FixedApp {
            installed: true,
            idx: None,
            compatible_addresses: vec![Some((0x48000, 0x20010000))],
            size: 0x2000,
        });

        let c_app_1: TockApp = TockApp::Flexible(FlexibleApp {
            installed: false,
            idx: None,
            size: 0x2000,
        });

        let apps: Vec<TockApp> = vec![rust_app_1, rust_app_2, rust_app_3, c_app_1];

        let reshuffled_apps = reshuffle_apps(settings, apps);

        let correct_config: Option<Vec<Index>> = Some(vec![
            Index {
                installed: true,
                idx: Some(0x0),
                ram_address: Some(0x20008000),
                address: 0x40000,
                size: 0x2000,
            },
            Index {
                installed: true,
                idx: Some(0x1),
                ram_address: Some(0x2000a000),
                address: 0x42000,
                size: 0x2000,
            },
            Index {
                installed: false,
                idx: Some(0x3),
                ram_address: None,
                address: 0x44000,
                size: 0x2000,
            },
            // padding
            Index {
                installed: false,
                idx: None,
                ram_address: None,
                address: 0x46000,
                size: 0x2000,
            },
            Index {
                installed: true,
                idx: Some(0x2),
                ram_address: Some(0x20010000),
                address: 0x48000,
                size: 0x2000,
            },
        ]);
        assert_eq!(reshuffled_apps, correct_config);
    }
}
