mod log_view;

use std::path::PathBuf;

use adw::prelude::*;
use gtk::gdk;
use relm4::prelude::*;
use relm4_icons::icon_names;
use tokio::sync::mpsc;
use vertx_simulator_ipc as ipc;

use self::log_view::LogView;
use crate::backpack::Backpack;
use crate::child::Child;

#[derive(Debug, Clone, Copy)]
pub(crate) enum NoMessages {}

#[derive(Debug)]
pub(crate) enum Message {
    BootPressed,
    ToggleConfigurator,
    Exited { restart: bool },
    Ipc(ipc::ToManager),
}

impl From<ipc::ToManager> for Message {
    fn from(message: ipc::ToManager) -> Self {
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
    backpack: Backpack,
}

impl App {
    fn start_vertx(&mut self, sender: ComponentSender<Self>) {
        let vertx = Child::new(
            &self.target_dir,
            self.boot_mode,
            &mut self.backpack,
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
            backpack: Backpack::new(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            Message::BootPressed => {
                // TODO: trigger VerTX power button when firmware is running
                self.start_vertx(sender);
            }
            Message::ToggleConfigurator => {
                if let Some(vertx) = &mut self.vertx {
                    vertx.send(ipc::ToVertx::ModeButtonPressed);
                }
            }

            Message::Exited { restart } => {
                let _ = self.vertx.take();

                if restart {
                    self.start_vertx(sender);
                } else {
                    self.boot_mode = 0;
                }
            }

            Message::Ipc(message) => match message {
                ipc::ToManager::SetBootMode(mode) => self.boot_mode = mode,
                ipc::ToManager::StatusLed { r, g, b } => {
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
