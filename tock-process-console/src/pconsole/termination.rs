// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OXIDOS AUTOMOTIVE 2024.

use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum Interrupted {
    OsSignal,
    UserRequest,
}

#[derive(Debug, Clone)]
pub struct Terminator {
    interrupt_sender: broadcast::Sender<Interrupted>,
}

impl Terminator {
    pub fn new(interrupt_sender: broadcast::Sender<Interrupted>) -> Self {
        Self { interrupt_sender }
    }

    pub fn terminate(&mut self, interrupted: Interrupted) -> anyhow::Result<()> {
        self.interrupt_sender.send(interrupted)?;

        Ok(())
    }
}

#[cfg(unix)]
async fn terminate_by_unix_signal(mut terminator: Terminator) {
    use tokio::signal::unix::signal;

    let mut interrupt_signal = signal(tokio::signal::unix::SignalKind::interrupt())
        .expect("failed to create interrupt signal stream");

    interrupt_signal.recv().await;

    terminator
        .terminate(Interrupted::OsSignal)
        .expect("failed to send interrupt signal");
}

pub fn create_terminator() -> (Terminator, broadcast::Receiver<Interrupted>) {
    let (tx, rx) = broadcast::channel(1);
    let terminator = Terminator::new(tx);

    #[cfg(unix)]
    tokio::spawn(terminate_by_unix_signal(terminator.clone()));

    (terminator, rx)
}
