// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OXIDOS AUTOMOTIVE 2024.

mod cli;
mod display;
mod known_boards;

use anyhow::{Context, Result};
use clap::ArgMatches;
use cli::make_cli;
use known_boards::KnownBoardNames;
use tockloader_lib::board_settings::BoardSettings;
use tockloader_lib::command_impl::install::InstallResolution;
use tockloader_lib::connection::{
    Connection, ProbeRSConnection, ProbeTargetInfo, SerialConnection, SerialTargetInfo,
    TockloaderConnection,
};
use tockloader_lib::known_boards::KnownBoard;
use tockloader_lib::tabs::tab::Tab;
use tockloader_lib::{
    list_debug_probes, list_serial_ports, CommandEraseApps, CommandInfo, CommandInstall,
    CommandList,
};

fn get_serial_target_info(user_options: &ArgMatches) -> SerialTargetInfo {
    let board = get_known_board(user_options);
    if let Some(board) = board {
        return board.serial_target_info();
    }

    let mut result = SerialTargetInfo::default();

    if let Some(baud_rate) = user_options.get_one::<u32>("baud-rate") {
        result.baud_rate = *baud_rate;
    }

    result
}

fn get_probe_target_info(user_options: &ArgMatches) -> ProbeTargetInfo {
    let board = get_known_board(user_options);
    if let Some(board) = board {
        return board.probe_target_info();
    }

    let chip = user_options
        .get_one::<String>("chip")
        .expect("Expected validation to catch missing chip")
        .clone();

    let mut result = ProbeTargetInfo::default(chip);

    if let Some(core) = user_options.get_one::<usize>("core") {
        result.core = *core;
    }

    result
}

fn get_board_settings(user_options: &ArgMatches) -> BoardSettings {
    let board = get_known_board(user_options);
    if let Some(board) = board {
        return board.get_settings();
    }

    let result = BoardSettings::default();

    if let Some(_strat_address_str) = user_options.get_one::<String>("app-address") {
        todo!()
    }

    result
}

fn using_serial(user_options: &ArgMatches) -> bool {
    let serial_flag = *user_options.get_one::<bool>("serial").unwrap_or(&false);
    if serial_flag {
        return true;
    }

    let is_listen_command = user_options
        .try_get_one::<String>("protocol")
        .is_ok_and(|option| option.is_some_and(|val| val == "legacy"));

    is_listen_command
}

fn get_known_board(user_options: &ArgMatches) -> Option<Box<dyn KnownBoard>> {
    user_options.get_one::<String>("board").map(|board| {
        match KnownBoardNames::from_str(board).expect("validation to ensure valid board") {
            KnownBoardNames::NucleoF4 => {
                Box::new(tockloader_lib::known_boards::NucleoF4) as Box<dyn KnownBoard>
            }
            KnownBoardNames::MicrobitV2 => {
                Box::new(tockloader_lib::known_boards::MicrobitV2) as Box<dyn KnownBoard>
            }
            KnownBoardNames::Nrf52840dk => {
                Box::new(tockloader_lib::known_boards::Nrf52840dk) as Box<dyn KnownBoard>
            }
            KnownBoardNames::NucleoU545ReQ => {
                Box::new(tockloader_lib::known_boards::NucleoU545ReQ) as Box<dyn KnownBoard>
            }
        }
    })
}

async fn open_connection(user_options: &ArgMatches) -> Result<TockloaderConnection> {
    if using_serial(user_options) {
        let path = if let Some(path) = user_options.get_one::<String>("port") {
            path.clone()
        } else {
            let serial_ports = list_serial_ports().context("Failed to list serial ports.")?;
            let port_names: Vec<_> = serial_ports.iter().map(|p| p.port_name.clone()).collect();

            inquire::Select::new("Which serial port do you want to use?", port_names)
                .prompt()
                .context("No device is connected.")?
        };

        let mut conn: TockloaderConnection = SerialConnection::new(
            path,
            get_serial_target_info(user_options),
            get_board_settings(user_options),
        )
        .into();
        conn.open()
            .await
            .context("Failed to open serial connection.")?;

        Ok(conn)
    } else {
        let ans =
            inquire::Select::new("Which debug probe do you want to use?", list_debug_probes())
                .prompt()
                .context("No debug probe is connected.")?;

        let mut conn: TockloaderConnection = ProbeRSConnection::new(
            ans,
            get_probe_target_info(user_options),
            get_board_settings(user_options),
        )
        .into();

        conn.open()
            .await
            .context("Failed to open probe connection.")?;

        Ok(conn)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut cmd = cli::make_cli();
    let matches = cmd.get_matches_mut();

    let user_level = matches
        .get_one::<String>("log-level")
        .map(String::as_str)
        .unwrap();

    let mut builder = env_logger::Builder::new();

    let cli_level = match user_level {
        "error" => log::LevelFilter::Error,
        "warn" => log::LevelFilter::Warn,
        "info" => log::LevelFilter::Info,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        level => panic!("Unknown log level: {level}"),
    };

    builder.filter_level(log::LevelFilter::Off);
    builder.filter_module("tockloader-lib", cli_level);
    builder.filter_module("tockloader", cli_level);
    builder.filter_module("tbf_parser", cli_level);

    builder.init();

    match matches.subcommand() {
        Some(("listen", sub_matches)) => {
            cli::validate(&mut cmd, sub_matches);
            let protocol = sub_matches
                .get_one::<String>("protocol")
                .map(String::as_str)
                .unwrap();
            match protocol {
                "legacy" => {
                    let conn = open_connection(sub_matches).await?;
                    match conn {
                        TockloaderConnection::ProbeRS(_) => panic!("Cannot establish connection."),
                        TockloaderConnection::Serial(serial_connection) => {
                            tock_process_console::legacy::run(
                                serial_connection
                                    .into_inner_stream()
                                    .expect("Expected board to be connected."),
                            )
                            .await;
                        }
                    }
                }
                "pconsole" => {
                    tock_process_console::pconsole::run()
                        .await
                        .context("Failed to run console.")?;
                }
                _ => {
                    panic!("Invalid protocol");
                }
            }
        }
        Some(("list", sub_matches)) => {
            cli::validate(&mut cmd, sub_matches);

            let mut conn = open_connection(sub_matches).await?;

            let app_details = conn.list().await.context("Failed to list apps.")?;

            display::print_list(&app_details).await;
        }
        Some(("info", sub_matches)) => {
            cli::validate(&mut cmd, sub_matches);
            let mut conn = open_connection(sub_matches).await?;

            let mut attributes = conn
                .info()
                .await
                .context("Failed to get data from the board.")?;

            display::print_info(&mut attributes.apps, &mut attributes.system).await;
        }
        Some(("install", sub_matches)) => {
            cli::validate(&mut cmd, sub_matches);
            let tab_file = Tab::open(sub_matches.get_one::<String>("tab").unwrap().to_string())
                .context("Failed to use provided tab file.")?;

            let no_replace = sub_matches.get_flag("no-replace");
            let resolution = if no_replace {
                InstallResolution::InstallAsNew
            } else {
                InstallResolution::Overwrite
            };

            let mut conn = open_connection(sub_matches).await?;

            conn.install_app(tab_file, resolution)
                .await
                .context("Failed to install app.")?;
        }
        Some(("erase-apps", sub_matches)) => {
            cli::validate(&mut cmd, sub_matches);
            let mut conn = open_connection(sub_matches).await?;

            conn.erase_apps().await.context("Failed to erase apps.")?;
        }
        _ => {
            println!("Could not run the provided subcommand.");
            _ = make_cli().print_help();
        }
    }
    Ok(())
}
