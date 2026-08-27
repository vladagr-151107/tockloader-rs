// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OXIDOS AUTOMOTIVE 2024.

use crate::board_settings::BoardSettings;
use crate::errors::{TabError, TockloaderError};
use crate::tabs::metadata::Metadata;
use std::fs::File;
use std::io::Read;
use tar::Archive;

pub struct TbfFile {
    pub filename: String,
    pub data: Vec<u8>,
}

pub struct Tab {
    metadata: Metadata,
    tbf_files: Vec<TbfFile>,
}

impl Tab {
    pub fn open(path: String) -> Result<Self, TockloaderError> {
        let mut metadata = None;
        let mut tbf_files = Vec::new();
        let tab_file = File::open(path).map_err(TabError::IO)?;
        let mut archive = Archive::new(tab_file);

        for archive_entry in archive.entries().map_err(TabError::IO)? {
            let mut archive_file = archive_entry.map_err(TabError::IO)?;

            let path = archive_file.path().map_err(TabError::IO)?;
            let file_name = match path.file_name().and_then(|name| name.to_str()) {
                Some(name) => name.to_owned(),
                None => continue,
            };

            if file_name == "metadata.toml" {
                let mut buf = String::new();
                archive_file
                    .read_to_string(&mut buf)
                    .map_err(TabError::IO)?;
                metadata = Some(Metadata::new(buf)?);
            } else if file_name.ends_with(".tbf") {
                let mut data = Vec::new();

                archive_file.read_to_end(&mut data).map_err(TabError::IO)?;
                tbf_files.push(TbfFile {
                    filename: file_name.to_string(),
                    data,
                });
            }
        }

        match metadata {
            Some(metadata) => Ok(Tab {
                metadata,
                tbf_files,
            }),
            None => Err(TabError::MissingMetadata.into()),
        }
    }

    pub fn is_compatible_with_kernel_verison(&self, _kernel_version: u32) -> bool {
        // Kernel version seems to not be working properly on the microbit bootloader. It is always
        // "1" despite the actual version.
        // return self.metadata.minimum_tock_kernel_version.major <= kernel_version;
        true
    }

    pub fn is_compatible_with_board(&self, board: &String) -> bool {
        if let Some(boards) = &self.metadata.only_for_boards {
            boards.contains(board)
        } else {
            true
        }
    }

    /// This function returns all the compatible binaries for a given tab
    ///
    /// Vector components:
    ///     - binary (Vec<u8>)
    ///     - start_address: u64
    ///     - ram_start_address: u64
    pub fn filter_tbfs(
        &self,
        settings: &BoardSettings,
    ) -> Result<Vec<Option<(u64, u64)>>, TockloaderError> {
        // save the file
        // also save flash start and ram start for comparing easily later
        let mut compatible_tbfs: Vec<Option<(u64, u64)>> = Vec::new();
        for file in &self.tbf_files {
            let (arch, flash, ram) = Self::split_arch(file.filename.to_string());
            // check if we have the same arch
            // check if flash and ram fit
            if flash != 0
                && ram != 0
                && arch.starts_with(settings.arch.as_ref().unwrap())
                && flash >= settings.app_start_address
            {
                log::info!("rust, pushed arch {arch}, flash {flash:#x}, ram {ram:#x}");
                compatible_tbfs.push(Some((flash, ram)));
            }

            // how about we don't do anything on else?
            // } else if arch.starts_with(settings.arch.as_ref().unwrap()) {
            //     // this happens for C apps, we'll have
            //     // arch = "cortex-m4.tbf"
            //     // without any flash and ram values
            //     compatible_tbfs.push(None);
            // }
        }
        Ok(compatible_tbfs)
    }

    fn split_arch(filename: String) -> (String, u64, u64) {
        // filename is always formatted like this:
        // "cortex-m0.0x10020000.0x20004000.tab"
        // splitting by .0x will give us "arch", "flash start", "ram start.tab"
        // 3 items
        // log::info!("filename {filename}");
        let data: Vec<&str> = filename.split(".0x").collect();
        if data.len() == 3 {
            let flashaddr: u64 = u64::from_str_radix(data[1], 16).unwrap();
            // split the ram address again because it also contains .tab
            // take the first item of the tuple
            let ramaddr: u64 = u64::from_str_radix(data[2].split_once(".").unwrap().0, 16).unwrap();
            (data[0].to_string(), flashaddr, ramaddr)
        } else {
            (data[0].to_string(), 0, 0)
        }
    }

    pub fn extract_binary(&self, arch: String) -> Result<Vec<u8>, TockloaderError> {
        for file in &self.tbf_files {
            if file.filename.starts_with(&arch) {
                return Ok(file.data.clone());
            }
        }

        Err(TabError::MissingBinary(arch.to_owned()).into())
    }

    pub fn name(&self) -> &str {
        &self.metadata.name
    }
}
