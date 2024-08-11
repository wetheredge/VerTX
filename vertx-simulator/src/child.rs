use std::path::Path;
use std::process::Stdio;
use std::{env, io};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{self, Command};
use tokio::sync::mpsc;
use vertx_simulator_ipc as ipc;

#[derive(Debug)]
pub(crate) struct Child {
    process: process::Child,
    ipc_tx: mpsc::UnboundedSender<ipc::ToFirmware>,
    stdin_abort: tokio::task::AbortHandle,
    stdout_abort: tokio::task::AbortHandle,
    stderr_abort: tokio::task::AbortHandle,
}

impl Child {
    pub(crate) fn new(
        target_dir: &Path,
        boot_mode: u8,
        log_tx: mpsc::UnboundedSender<String>,
        sender: relm4::ComponentSender<crate::ui::App>,
    ) -> io::Result<Self> {
        let path = target_dir.join("debug/simulator");
        let path = path.canonicalize()?.into_os_string();

        let mut vertx =
            Command::new(path)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .env_clear()
                .env("VERTX_CONFIG", "config.bin")
                .envs(env::vars_os().filter(|(name, _)| {
                    name.to_str().is_some_and(|name| name.starts_with("VERTX_"))
                }))
                .env("VERTX_BOOT_MODE", boot_mode.to_string().as_str())
                .spawn()?;

        let mut stdin = vertx.stdin.take().unwrap();
        let stdout = vertx.stdout.take().unwrap();
        let stderr = vertx.stderr.take().unwrap();

        let (tx, mut stdin_rx) = mpsc::unbounded_channel();
        let stdin_abort = tokio::spawn(async move {
            while let Some(message) = stdin_rx.recv().await {
                let bytes = ipc::serialize(&message).unwrap();
                stdin.write_all(&bytes).await.unwrap();
                stdin.flush().await.unwrap();
            }
        })
        .abort_handle();

        let stdout_abort = tokio::spawn(async move {
            let mut stdout = BufReader::new(stdout);
            let mut buffer = Vec::new();
            while stdout.read_until(0, &mut buffer).await.unwrap() > 0 {
                let message: ipc::ToManager = ipc::deserialize(&mut buffer).unwrap();
                sender.input_sender().send(message.into()).unwrap();
                buffer.clear();
            }
        })
        .abort_handle();

        let stderr_abort = tokio::spawn(async move {
            let mut stderr = tokio::io::BufReader::new(stderr).lines();
            while let Some(line) = stderr.next_line().await.unwrap() {
                log_tx.send(line).unwrap();
            }
        })
        .abort_handle();

        Ok(Child {
            process: vertx,
            ipc_tx: tx,
            stdin_abort,
            stdout_abort,
            stderr_abort,
        })
    }

    pub(crate) fn send(&self, message: ipc::ToFirmware) {
        self.ipc_tx.send(message).unwrap();
    }

    pub(crate) fn has_exited(&mut self) -> bool {
        self.process.try_wait().unwrap().is_some()
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        self.process.start_kill().unwrap();
        self.stdin_abort.abort();
        self.stdout_abort.abort();
        self.stderr_abort.abort();
    }
}
