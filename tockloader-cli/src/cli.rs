// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OXIDOS AUTOMOTIVE 2024.

use clap::error::ErrorKind;
use clap::{arg, crate_version, value_parser, ArgMatches, Command};

use crate::known_boards::list_known_board_names;

/// Create the [command](clap::Command) object which will handle all of the command line arguments.
pub fn make_cli() -> Command {
    Command::new("tockloader")
        .about("This is a sample description.")
        .version(crate_version!())
        .subcommand_required(true)
        .subcommands(get_subcommands())
        .arg(
            arg!(--"log-level" <LEVEL>)
                .required(false)
                .value_parser(["error", "warn", "info", "debug", "trace"])
                .default_value("info")
                .global(true),
        )
    // Note: arg_require_else_help will trigger the help command if no argument/subcommand is given.
    // This means that the --debug flag will not trigger the help menu, even if alone it does nothing.
}

/// Generate all of the [subcommands](clap::Command) used by the program.
fn get_subcommands() -> Vec<Command> {
    vec![
        Command::new("listen")
            .about("Open a terminal to receive UART data")
            .args(get_channel_args())
            .args([
                // TODO(eva-cosma): Change default to pconsole when new format is implemented and adopted into tock
                arg!(--protocol <PROTOCOL> "Choose between legacy and pconsole protocol")
                    .default_value("legacy")
                    .value_parser(["legacy", "pconsole"]),
            ])
            .arg_required_else_help(false),
        Command::new("list")
            .about("List and inspect probes")
            .args(get_app_args())
            .args(get_channel_args())
            .arg_required_else_help(false),
        Command::new("info")
            .about("Verbose information about the connected board")
            .args(get_app_args())
            .args(get_channel_args())
            .arg_required_else_help(false),
        Command::new("install")
            .about("Install apps")
            .args(get_app_args())
            .args(get_channel_args())
            .arg_required_else_help(false),
        Command::new("erase-apps")
            .about("Erase apps")
            .args(get_app_args())
            .args(get_channel_args())
            .arg_required_else_help(false),
    ]
}

/// Generate all of the [arguments](clap::Arg) that are required by subcommands which work with apps.
fn get_app_args() -> Vec<clap::Arg> {
    let probe_args_ids = get_probe_args().into_iter().map(|arg| arg.get_id().clone());

    vec![
        // Default of ProbeTargetInfo: 0x00030000
        arg!(-a --"app-address" <ADDRESS> "Address where apps are located")
            .conflicts_with_all(probe_args_ids.clone().collect::<Vec<_>>()),
        arg!(--tab <TAB> "Specify the path of the tab file"),
        arg!(--"no-replace" "Install alongside an existing app of the same name instead of overwriting it")
            .action(clap::ArgAction::SetTrue),
    ]
    // Note: the .action(clap::ArgAction::SetTrue) doesn't seem to be necessary, though in clap documentation it is used.
}

/// Generate all of the [arguments](clap::Arg) that are required by subcommands which work
/// with channels and computer-board communication.
fn get_channel_args() -> Vec<clap::Arg> {
    let probe_args_ids = get_probe_args_ids().into_iter();
    let serial_args_ids = get_serial_args_ids().into_iter();

    let known_board_names = list_known_board_names()
        .into_iter()
        .map(|x| x.to_str())
        .collect::<Vec<_>>();

    vec![
        arg!(--serial "Use the serial bootloader to flash")
            .action(clap::ArgAction::SetTrue)
            .conflicts_with_all(probe_args_ids.clone().collect::<Vec<_>>()),
        arg!(--board <BOARD> "Explicitly specify the board that is being targeted")
            .value_parser(known_board_names)
            .conflicts_with_all(
                serial_args_ids
                    .clone()
                    .chain(probe_args_ids.clone())
                    .collect::<Vec<_>>(),
            ),
    ]
    .into_iter()
    .chain(get_probe_args())
    .chain(get_serial_args())
    .collect()
}

fn get_probe_args() -> Vec<clap::Arg> {
    let serial_args_ids = get_serial_args_ids().into_iter();

    vec![
        // Conditionally required via custom validation
        arg!(--chip <CHIP> "Explicitly specify the chip"),
        // Default of ProbeTargetInfo: 0
        arg!(--core <CORE> "Explicitly specify the core").value_parser(clap::value_parser!(usize)),
    ]
    .into_iter()
    .map(|arg| arg.conflicts_with_all(serial_args_ids.clone().collect::<Vec<_>>()))
    .map(|arg| arg.help_heading("Probe Connection Options"))
    .collect::<Vec<_>>()
}

fn get_probe_args_ids() -> Vec<clap::Id> {
    vec!["chip".into(), "core".into()]
}

fn get_serial_args() -> Vec<clap::Arg> {
    let probe_args_ids = get_probe_args_ids().into_iter();

    vec![
        arg!(-p --port <PORT> "The serial port or device name to use"),
        // Default of SerialTargetInfo: 115200
        arg!(--"baud-rate" <RATE> "If using serial, set the target baud rate")
            .value_parser(value_parser!(u32)),
        // TODO: add more serial arguments to match with SerialTargetInfo
    ]
    .into_iter()
    .map(|arg| arg.conflicts_with_all(probe_args_ids.clone().collect::<Vec<_>>()))
    .map(|arg| arg.help_heading("Serial Connection Options"))
    .collect::<Vec<_>>()
}

fn get_serial_args_ids() -> Vec<clap::Id> {
    vec!["port".into(), "baud-rate".into()]
}

pub fn validate(cmd: &mut Command, user_options: &ArgMatches) {
    // Make 'chip' required if not using serial or board
    if user_options.get_one::<String>("chip").is_none()
        && !user_options.get_one::<bool>("serial").unwrap_or(&false)
        && user_options.get_one::<String>("board").is_none()
    {
        cmd.error(
            ErrorKind::MissingRequiredArgument,
            "the argument '--chip' is required for probe connections. This can be inferred using a known board ('--board').",
        )
        .exit();
    }
}

mod test {
    #[test]
    fn ids_match_with_args() {
        use super::*;

        let mut probe_args_ids = get_probe_args_ids();
        let mut serial_args_ids = get_serial_args_ids();

        let mut probe_args = get_probe_args()
            .into_iter()
            .map(|arg| arg.get_id().clone())
            .collect::<Vec<_>>();

        let mut serial_args = get_serial_args()
            .into_iter()
            .map(|arg| arg.get_id().clone())
            .collect::<Vec<_>>();

        probe_args_ids.sort();
        serial_args_ids.sort();

        probe_args.sort();
        serial_args.sort();

        assert_eq!(probe_args_ids, probe_args);
        assert_eq!(serial_args_ids, serial_args);
    }
}
