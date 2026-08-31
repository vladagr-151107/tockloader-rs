// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OXIDOS AUTOMOTIVE 2024.

use anyhow::Error;
use bytes::Bytes;
use tokio::io::{split, AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio_serial::{SerialPortBuilderExt, SerialStream};

use super::decoder::{BoardMessage, Decoder};
use super::event::{Event, NewMessageEvent};

pub struct ConnectionHandler {
    port_reader: ReadHalf<SerialStream>,
    port_writer: WriteHalf<SerialStream>,
    event_writer: UnboundedSender<Event>,
    command_reader: UnboundedReceiver<Bytes>,
    decoder: Decoder,
    decoded_receiver: UnboundedReceiver<BoardMessage>,
}

impl ConnectionHandler {
    pub async fn connection_init(
        tty: &str,
    ) -> Result<(UnboundedReceiver<Event>, UnboundedSender<Bytes>), Error> {
        let mut port: SerialStream = tokio_serial::new(tty, 115200).open_native_async()?;

        #[cfg(unix)]
        port.set_exclusive(false)?;

        let (port_reader, port_writer) = split(port);

        let (event_writer, event_reader) = mpsc::unbounded_channel::<Event>();
        let (command_writer, command_reader) = mpsc::unbounded_channel::<Bytes>();

        // let (encoded_sender, encoded_receiver) = mpsc::unbounded_channel::<Bytes>();
        let (decoded_sender, decoded_receiver) = mpsc::unbounded_channel::<BoardMessage>();
        let decoder = Decoder::new(decoded_sender);

        let handler = ConnectionHandler {
            port_reader,
            port_writer,
            event_writer,
            command_reader,
            decoder,
            decoded_receiver,
        };

        handler.start().await;
        Ok((event_reader, command_writer))
    }

    pub async fn start(mut self) {
        tokio::spawn(async move {
            let mut buffer = [0u8; 4096];

            loop {
                tokio::select! {
                    // We receive a command from the user
                    Some(command) = self.command_reader.recv() => {
                        let _ = self.port_writer.write(&command).await;
                    },
                    // We read something from the serial port
                    port_read_result = self.port_reader.read(&mut buffer) => {
                        match port_read_result {
                            Ok(len) => {
                                let vec = buffer[..len].to_vec();
                                self.decoder.send_encoded_message(vec);
                            },
                            // We encountered an error while reading from the serial port
                            // Means thath we lost the connection
                            Err(err) => {
                                let _ = self.event_writer.send(
                                    Event::LostConnection(err.to_string())
                                );
                            }
                        }
                    },
                    received_decoded_result = self.decoded_receiver.recv() => {
                        if let Some(board_message) = received_decoded_result {
                            let app_name = if board_message.is_app {
                                format!("app_{}", board_message.pid)
                            } else if board_message.pid == 0 {
                                String::from("debug")
                            } else {
                                String::from("kernel")
                            };

                            let _ = self.event_writer.send(
                                Event::NewMessage(
                                    NewMessageEvent{
                                        app: app_name,
                                        pid: board_message.pid,
                                        is_app: board_message.is_app,
                                        payload: board_message.payload
                                    }
                                )
                            );
                        }
                    }
                }
            }
        });
    }
}
