use std::path::Path;
use std::process::Stdio;
use std::{env, io};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use vertx_simulator_ipc as ipc;

use crate::backpack::Backpack;
use crate::ui;

#[derive(Debug)]
pub(crate) struct Child {
    tx: mpsc::UnboundedSender<ipc::Message<'static, ipc::ToVertx>>,
    abort: [tokio::task::AbortHandle; 4],
}

impl Child {
    pub(crate) fn new(
        target_dir: &Path,
        boot_mode: u8,
        backpack: &mut Backpack,
        log_tx: mpsc::UnboundedSender<String>,
        sender: relm4::ComponentSender<ui::App>,
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
                .env("VERTX_BOOT_MODE", boot_mode.to_string())
                .kill_on_drop(true)
                .spawn()?;

        let mut stdin = vertx.stdin.take().unwrap();
        let stdout = vertx.stdout.take().unwrap();
        let stderr = vertx.stderr.take().unwrap();

        let abort_exit = {
            let sender = sender.clone();
            tokio::spawn(async move {
                let code = vertx.wait().await.unwrap().code().unwrap();

                sender
                    .input_sender()
                    .send(ui::Message::Exited {
                        restart: code == ipc::EXIT_REBOOT,
                    })
                    .unwrap();
            })
            .abort_handle()
        };

        let (abort_stdin, tx) = {
            let (tx, mut rx) = mpsc::unbounded_channel::<ipc::Message<'static, ipc::ToVertx>>();

            let task = tokio::spawn(async move {
                while let Some(message) = rx.recv().await {
                    let message = ipc::serialize(&message).unwrap();
                    stdin.write_all(&message).await.unwrap();
                    stdin.flush().await.unwrap();
                }
            });

            (task.abort_handle(), tx)
        };

        let (abort_stdout, backpack_rx) = {
            let (backpack_tx, backpack_rx) = mpsc::unbounded_channel::<Vec<u8>>();

            let task = &tokio::spawn(async move {
                let mut stdout = BufReader::new(stdout);
                let mut buffer = Vec::new();
                while stdout.read_until(0, &mut buffer).await.unwrap() > 0 {
                    let message: ipc::Message<'_, ipc::ToManager> =
                        ipc::deserialize(&mut buffer).unwrap();

                    match message {
                        ipc::Message::Simulator(message) => {
                            sender.input_sender().send(message.into()).unwrap();
                        }
                        ipc::Message::Backpack(chunk) => {
                            backpack_tx.send(chunk.into_owned()).unwrap();
                        }
                    }

                    buffer.clear();
                }
            });

            (task.abort_handle(), backpack_rx)
        };

        let abort_stderr = tokio::spawn(async move {
            let mut stderr = tokio::io::BufReader::new(stderr).lines();
            while let Some(line) = stderr.next_line().await.unwrap() {
                log_tx.send(line).unwrap();
            }
        })
        .abort_handle();

        backpack.start(tx.clone(), backpack_rx);

        Ok(Self {
            tx,
            abort: [abort_exit, abort_stdin, abort_stdout, abort_stderr],
        })
    }

    pub(crate) fn send(&self, message: ipc::ToVertx) {
        self.tx.send(ipc::Message::Simulator(message)).unwrap();
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        for task in &self.abort {
            task.abort();
        }
    }
}
