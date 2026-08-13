// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OXIDOS AUTOMOTIVE 2024.

// The "X" commands are for external flash

use crate::errors::{self, InternalError, TockError};
use bytes::BytesMut;
use errors::TockloaderError;
use log::trace;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_serial::{SerialPort, SerialStream};

// Tell the bootloader to reset its buffer to handle a new command
pub const SYNC_MESSAGE: [u8; 3] = [0x00, 0xFC, 0x05];

// "This was chosen as it is infrequent in .bin files" - immesys
pub const ESCAPE_CHAR: u8 = 0xFC;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(5000);

#[allow(dead_code)]
pub enum Command {
    // Commands from this tool to the bootloader
    Ping = 0x01,
    Info = 0x03,
    ID = 0x04,
    Reset = 0x05,
    ErasePage = 0x06,
    WritePage = 0x07,
    XEBlock = 0x08,
    XWPage = 0x09,
    Crcx = 0x10,
    ReadRange = 0x11,
    XRRange = 0x12,
    SetAttribute = 0x13,
    GetAttribute = 0x14,
    CRCInternalFlash = 0x15,
    Crcef = 0x16,
    XEPage = 0x17,
    XFinit = 0x18,
    ClkOut = 0x19,
    WUser = 0x20,
    ChangeBaudRate = 0x21,
    Exit = 0x22,
    SetStartAddress = 0x23,
}

#[derive(Clone, Debug)]
pub enum Response {
    // Responses from the bootloader
    Overflow = 0x10,
    Pong = 0x11,
    BadAddr = 0x12,
    IntError = 0x13,
    BadArgs = 0x14,
    OK = 0x15,
    Unknown = 0x16,
    XFTimeout = 0x17,
    Xfepe = 0x18,
    Crcrx = 0x19,
    ReadRange = 0x20,
    XRRange = 0x21,
    GetAttribute = 0x22,
    CRCInternalFlash = 0x23,
    Crcxf = 0x24,
    Info = 0x25,
    ChangeBaudFail = 0x26,
    BadResp,
}

impl From<u8> for Response {
    fn from(value: u8) -> Self {
        match value {
            0x10 => Response::Overflow,
            0x11 => Response::Pong,
            0x12 => Response::BadAddr,
            0x13 => Response::IntError,
            0x14 => Response::BadArgs,
            0x15 => Response::OK,
            0x16 => Response::Unknown,
            0x17 => Response::XFTimeout,
            0x18 => Response::Xfepe,
            0x19 => Response::Crcrx,
            0x20 => Response::ReadRange,
            0x21 => Response::XRRange,
            0x22 => Response::GetAttribute,
            0x23 => Response::CRCInternalFlash,
            0x24 => Response::Crcxf,
            0x25 => Response::Info,
            0x26 => Response::ChangeBaudFail,

            // This error handling is temmporary
            //TODO(Micu Ana): Add error handling
            _ => Response::BadResp,
        }
    }
}

#[allow(dead_code)]
pub async fn toggle_bootloader_entry_dtr_rts(
    port: &mut SerialStream,
) -> Result<(), TockloaderError> {
    port.write_data_terminal_ready(true)?;
    port.write_request_to_send(true)?;

    tokio::time::sleep(Duration::from_millis(100)).await;

    port.write_data_terminal_ready(false)?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    port.write_request_to_send(false)?;

    Ok(())
}

async fn read_bytes(
    port: &mut SerialStream,
    bytes_to_read: usize,
    timeout: Duration,
) -> Result<BytesMut, TockloaderError> {
    let mut ret = BytesMut::with_capacity(bytes_to_read);
    let mut read_bytes = 0;

    tokio::time::timeout(timeout, async {
        while read_bytes < bytes_to_read {
            read_bytes += port
                .read_buf(&mut ret)
                .await
                .map_err(|e| TockloaderError::Serial(e.into()))?;
        }
        Ok(ret)
    })
    .await
    .map_err(|_| TockError::BootloaderTimeout)?
}

async fn write_bytes(
    port: &mut SerialStream,
    bytes: &[u8],
    timeout: Duration,
) -> Result<(), TockloaderError> {
    let mut bytes_written = 0;

    tokio::time::timeout(timeout, async {
        while bytes_written != bytes.len() {
            bytes_written += port
                .write_buf(&mut &bytes[bytes_written..])
                .await
                .map_err(|e| TockloaderError::Serial(e.into()))?;
        }
        Ok(())
    })
    .await
    .map_err(|_| TockError::BootloaderTimeout)?
}

pub async fn ping_bootloader_and_wait_for_response(
    port: &mut SerialStream,
) -> Result<(), TockloaderError> {
    let ping_pkt = [ESCAPE_CHAR, Command::Ping as u8];

    for attempt in 0..30 {
        trace!("Attempt {} at connecting to the bootloader", attempt + 1);
        write_bytes(port, &ping_pkt, DEFAULT_TIMEOUT).await?;
        let ret = match read_bytes(port, 2, DEFAULT_TIMEOUT).await {
            Ok(buf) => buf,
            Err(_) => continue,
        };

        if ret[1] == Response::Pong as u8 {
            return Ok(());
        }
    }

    Err(InternalError::BootloaderNotPresent.into())
}

#[allow(dead_code)]
pub async fn issue_command(
    port: &mut SerialStream,
    command: Command,
    mut message: Vec<u8>,
    sync: bool,
    response_len: usize,
    response_code: Response,
) -> Result<(Response, Vec<u8>), TockloaderError> {
    // Setup a command to send to the bootloader and handle the response
    // Generate the message to send to the bootloader
    let mut i = 0;
    while i < message.len() {
        if message[i] == ESCAPE_CHAR {
            // Escaped by replacing all 0xFC with two consecutive 0xFC - tock bootloader readme
            message.insert(i + 1, ESCAPE_CHAR);
            // Skip the inserted character
            i += 2;
        } else {
            i += 1;
        }
    }
    message.push(ESCAPE_CHAR);
    message.push(command as u8);

    // If there should be a sync/reset message, prepend the outgoing message with it
    if sync {
        message.insert(0, SYNC_MESSAGE[0]);
        message.insert(1, SYNC_MESSAGE[1]);
        message.insert(2, SYNC_MESSAGE[2]);
    }

    // Write the command message
    write_bytes(port, &message, DEFAULT_TIMEOUT).await?;

    // Response has a two byte header, then response_len bytes
    let header = read_bytes(port, 2, DEFAULT_TIMEOUT).await?;

    if header[0..2] != [ESCAPE_CHAR, response_code as u8] {
        return Err(TockError::BootloaderBadHeader(header[0], header[1]).into());
    }

    if response_len != 0 {
        let input = read_bytes(port, response_len, DEFAULT_TIMEOUT).await?;
        let mut result = Vec::with_capacity(input.len());

        // De-escape and add array of read in the bytes

        // TODO(eva-cosma): Extract this into a function and unit test this.
        let mut i = 0;
        while i < input.len() {
            if i + 1 < input.len() && input[i] == ESCAPE_CHAR && input[i + 1] == ESCAPE_CHAR {
                // Found consecutive ESCAPE_CHAR bytes, add only one
                result.push(ESCAPE_CHAR);
                i += 2; // Skip both bytes
            } else {
                // Not consecutive ESCAPE_CHAR, add the current byte
                result.push(input[i]);
                i += 1;
            }
        }

        Ok((Response::from(header[1]), result))
    } else {
        Ok((Response::from(header[1]), vec![]))
    }
}
