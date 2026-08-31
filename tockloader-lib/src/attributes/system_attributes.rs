// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OXIDOS AUTOMOTIVE 2024.

use byteorder::{ByteOrder, LittleEndian};

use super::decode::DecodedAttribute;
use crate::errors::{AttributeParseError, TockError, TockloaderError};
use crate::IO;

/// This structure contains all relevant information about board that is stored
/// in the bootloader ROM.
///
/// Note: not all system attributes are present on all boards. You cannot assume
/// any of these structure members are `Some(_)`.
///
/// See also <https://book.tockos.org/doc/kernel_attributes.html?highlight=attributes#header-format>
#[derive(Debug)]
pub struct SystemAttributes {
    pub board: Option<String>,
    pub arch: Option<String>,
    pub appaddr: Option<u64>,
    pub boothash: Option<String>,
    pub bootloader_version: Option<String>,
    pub sentinel: Option<String>,
    pub kernel_version: Option<u64>,
    pub app_mem_start: Option<u32>,
    pub app_mem_len: Option<u32>,
    pub kernel_bin_start: Option<u32>,
    pub kernel_bin_len: Option<u32>,
}

impl SystemAttributes {
    pub(crate) fn new() -> SystemAttributes {
        SystemAttributes {
            board: None,
            arch: None,
            appaddr: None,
            boothash: None,
            bootloader_version: None,
            sentinel: None,
            kernel_version: None,
            app_mem_start: None,
            app_mem_len: None,
            kernel_bin_start: None,
            kernel_bin_len: None,
        }
    }

    /// Check if the bootloader is present on the version of Tock
    pub(crate) async fn bootloader_is_present(
        conn: &mut dyn IO,
        flash_start_address: u64,
    ) -> Result<bool, TockloaderError> {
        // If a bootloader is present, the start of 0x400 will read
        // "TOCKBOOTLOADER", which is exactly 14 bytes long, so we'll read
        // 14 bytes starting from 0x400.
        let flag = conn.read(flash_start_address + 0x400, 14).await?;
        if flag == "TOCKBOOTLOADER".as_bytes() {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Read system and kernel attributes.
    /// System attributes are read only if the board has
    /// a bootloader present.
    /// # Parameters
    /// - `conn` : Either a SerialConnection or a ProbeRSConnection.
    /// - `flash_address` : The start of flash memory.
    /// - `start_address` : The start of User Space Applications in flash memory.
    /// # Returns
    /// - Ok(result): if attributes were read successfully
    /// - Err(TockloaderError::MisconfiguredBoard): if no start address is found or valid
    /// - Err(TockloaderError::MisconfiguredBoard): if attributes don't follow the UTF-8 format
    /// - Err(TockloaderError::ProbeRsReadError): if reading fails on ProbeRS
    /// - Err(TockloaderError::SerialReadError): if reading fails on Serial
    pub(crate) async fn read_system_attributes(
        conn: &mut dyn IO,
        flash_start_address: u64,
        app_start_address: u64,
    ) -> Result<SystemAttributes, TockloaderError> {
        let mut result = SystemAttributes::new();
        // System attributes start at an offset of 0x600 and up to 0x9FF. See:
        // https://book.tockos.org/doc/memory_layout#flash-1
        //
        let address = flash_start_address + 0x600;

        let buf = conn.read(address, 64 * 16).await?;

        let has_bootloader = Self::bootloader_is_present(conn, flash_start_address).await?;

        if !has_bootloader {
            // TODO(K-Nicolas-10): Chain: CLI arguments -> hardcoded start_address -> calculate from kernel app start address
            result.appaddr = Some(app_start_address);
        } else {
            let mut attribute_chunks = buf
                .as_chunks::<{ DecodedAttribute::ENCODED_LEN }>()
                .0
                .iter();
            // We always read 16 * 64 bytes so we won't panic
            if let Some(board) = attribute_chunks
                .next()
                .and_then(|c| DecodedAttribute::decode(c))
            {
                result.board = Some(board.value);
            }

            if let Some(arch) = attribute_chunks
                .next()
                .and_then(|c| DecodedAttribute::decode(c))
            {
                result.arch = Some(arch.value);
            }

            if let Some(appaddr) = attribute_chunks
                .next()
                .and_then(|c| DecodedAttribute::decode(c))
            {
                result.appaddr = Some(
                    u64::from_str_radix(appaddr.value.trim_start_matches("0x"), 16).map_err(
                        |e| TockError::AttributeParsing(AttributeParseError::InvalidNumber(e)),
                    )?,
                );
            }

            if let Some(boothash) = attribute_chunks
                .next()
                .and_then(|c| DecodedAttribute::decode(c))
            {
                result.boothash = Some(boothash.value);
            }
            // At this address lives the version of the bootloader we are using.
            // Trying to read this when no bootloader is present will result in a panic.
            // So we first verify if we have a bootloader, if not we just skip this and
            // return None for the bootloader version.
            let address = flash_start_address + 0x40E;

            let buf = conn.read(address, 8).await?;

            let string = String::from_utf8(buf.to_vec())
                .map_err(|e| TockError::AttributeParsing(AttributeParseError::InvalidString(e)))?;

            let string = string.trim_matches(char::from(0));

            result.bootloader_version = Some(string.to_owned());
        }

        // TODO(eva-cosma): separate kernel attributes from kernel flags.

        // The 100 bytes prior to the application start address are reserved for the kernel attributes and flags
        let kernel_attr_addr = result
            .appaddr
            .ok_or(TockError::MissingAttribute("appaddr".to_owned()))?
            - 100;
        let kernel_attr_binary = conn.read(kernel_attr_addr, 100).await?;

        let sentinel = std::str::from_utf8(&kernel_attr_binary[96..100])
            .map_err(|_| TockError::AttributeParsing(AttributeParseError::InvalidSentinel))?
            .to_string();

        if sentinel != "TOCK" {
            return Err(TockError::AttributeParsing(AttributeParseError::InvalidSentinel).into());
        }
        let kernel_version = LittleEndian::read_uint(&kernel_attr_binary[95..96], 1);

        let app_memory_len = LittleEndian::read_u32(&kernel_attr_binary[84..92]);
        let app_memory_start = LittleEndian::read_u32(&kernel_attr_binary[80..84]);

        let kernel_binary_start = LittleEndian::read_u32(&kernel_attr_binary[68..72]);
        let kernel_binary_len = LittleEndian::read_u32(&kernel_attr_binary[72..76]);

        result.sentinel = Some(sentinel);
        result.kernel_version = Some(kernel_version);
        result.app_mem_start = Some(app_memory_start);
        result.app_mem_len = Some(app_memory_len);
        result.kernel_bin_start = Some(kernel_binary_start);
        result.kernel_bin_len = Some(kernel_binary_len);

        Ok(result)
    }
}
