mod log_view;

use std::path::PathBuf;

use adw::prelude::*;
use gtk::gdk;
use relm4::prelude::*;
use relm4_icons::icon_names;
use tokio::sync::mpsc;
use vertx_simulator_ipc::{ToFirmware, ToManager};

use self::log_view::LogView;
use crate::child::Child;

#[derive(Debug, Clone, Copy)]
pub(crate) enum NoMessages {}

#[derive(Debug)]
pub(crate) enum Message {
    BootPressed,
    ToggleConfigurator,

    Ipc(ToManager),
}

impl From<ToManager> for Message {
    fn from(message: ToManager) -> Self {
        Self::Ipc(message)
    }
}

#[derive(Debug)]
pub(crate) struct App {
    target_dir: PathBuf,

    log_view: Controller<LogView>,

    boot_mode: u8,
    status_color: gdk::RGBA,
    log_tx: mpsc::UnboundedSender<String>,
    vertx: Option<Child>,
}

impl App {
    fn start_vertx(&mut self, sender: ComponentSender<Self>) {
        let vertx = Child::new(
            &self.target_dir,
            self.boot_mode,
            self.log_tx.clone(),
            sender,
        );
        self.vertx = Some(vertx.unwrap());
    }
}

#[relm4::component(pub(crate))]
impl SimpleComponent for App {
    type Init = ();
    type Input = Message;
    type Output = NoMessages;

    view! {
        window = adw::ApplicationWindow {
            set_title: Some("VerTX Simulator"),
            set_default_size: (600, 300),

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar::new() {
                    pack_end = &gtk::Button {
                        set_icon_name: icon_names::SETTINGS,
                        set_tooltip: "Toggle VerTX configurator mode",
                        connect_clicked => Message::ToggleConfigurator,
                    },
                    pack_end = &gtk::Button {
                        set_icon_name: "system-shutdown",
                        connect_clicked => Message::BootPressed,
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 6,
                    set_margin_all: 6,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 6,

                        gtk::Label {
                            set_label: "Status LED",
                        },
                        gtk::ColorDialogButton {
                            set_sensitive: false,
                            #[watch]
                            set_rgba: &model.status_color,
                        },
                    },

                    model.log_view.widget(),
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let (log_tx, log_rx) = mpsc::unbounded_channel();

        let log_view = LogView::builder()
            .launch(log_rx)
            .forward(sender.input_sender(), |msg| match msg {});

        let model = Self {
            target_dir: crate::get_target_dir(),
            boot_mode: 0,
            status_color: gdk::RGBA::new(0., 0., 0., 1.),
            log_view,
            log_tx,
            vertx: None,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            Message::BootPressed => {
                // TODO: trigger VerTX power button when firmware is running

                let should_boot = match self.vertx {
                    Some(ref mut vertx) => vertx.has_exited(),
                    None => true,
                };

                if should_boot {
                    let vertx = Child::new(
                        &self.target_dir,
                        self.boot_mode,
                        self.log_tx.clone(),
                        sender,
                    );
                    self.vertx = Some(vertx.unwrap());
                }
            }
            Message::ToggleConfigurator => {
                if let Some(vertx) = &mut self.vertx {
                    vertx.send(ToFirmware::ModeButtonPressed);
                }
            }

            Message::Ipc(message) => match message {
                ToManager::ChangeMode(mode) => self.boot_mode = mode,
                ToManager::ShutDown => {
                    // TODO: wait for exit?
                    let _ = self.vertx.take();
                }
                ToManager::Reboot => {
                    // TODO: wait for exit?
                    let _ = self.vertx.take();
                    self.start_vertx(sender);
                }
                ToManager::StatusLed { r, g, b } => {
                    self.status_color = gdk::RGBA::new(
                        f32::from(r) / 255.,
                        f32::from(g) / 255.,
                        f32::from(b) / 255.,
                        1.,
                    );
                }
            },
        }
    }
}
