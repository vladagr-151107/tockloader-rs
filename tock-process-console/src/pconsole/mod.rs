// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OXIDOS AUTOMOTIVE 2024.

mod board;
mod state_store;
mod termination;
mod ui_management;

use state_store::StateStore;
use termination::{create_terminator, Interrupted};
use ui_management::UiManager;

pub async fn run() -> anyhow::Result<()> {
    let (terminator, mut interrupt_reader) = create_terminator();
    let (state_store, state_reader) = StateStore::new();
    let (ui_manager, action_reader) = UiManager::new();

    tokio::try_join!(
        state_store.main_loop(terminator, action_reader, interrupt_reader.resubscribe()),
        ui_manager.main_loop(state_reader, interrupt_reader.resubscribe(),)
    )?;

    if let Ok(reason) = interrupt_reader.recv().await {
        match reason {
            Interrupted::UserRequest => println!("Exited per user request"),
            Interrupted::OsSignal => println!("Exited because of os signal"),
        }
    } else {
        println!("Exited due to an unexpected error");
    }

    Ok(())
}
