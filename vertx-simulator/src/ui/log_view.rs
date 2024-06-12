use std::cell::Cell;
use std::rc::Rc;

use gtk::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;
use tokio::sync::mpsc;

use super::NoMessages;

#[derive(Debug)]
pub(super) struct LogView {
    rows: FactoryVecDeque<LogViewRow>,
    is_at_bottom: Rc<Cell<bool>>,
}

#[relm4::component(pub(super))]
impl Component for LogView {
    type CommandOutput = String;
    type Init = mpsc::UnboundedReceiver<String>;
    type Input = NoMessages;
    type Output = NoMessages;

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_hexpand: true,
            set_vexpand: true,

            #[local_ref]
            row_box -> gtk::ListBox {
                set_selection_mode: gtk::SelectionMode::None,
            }
        }
    }

    fn init(
        mut log_rx: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let rows = FactoryVecDeque::builder()
            .launch_default()
            .forward(sender.input_sender(), |msg| match msg {});

        let model = LogView {
            rows,
            is_at_bottom: Rc::new(Cell::new(true)),
        };

        let row_box = model.rows.widget();
        let widgets = view_output!();

        let adj = root.vadjustment();
        adj.connect_upper_notify({
            let is_at_bottom = model.is_at_bottom.clone();
            move |adj| {
                if is_at_bottom.get() {
                    adj.set_value(adj.upper());
                }
            }
        });

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    loop {
                        let line = log_rx.recv().await.unwrap();
                        out.send(line).unwrap();
                    }
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        let adj = root.vadjustment();
        self.is_at_bottom
            .replace((adj.value() + adj.page_size()) >= (adj.upper() - 1.));

        self.rows.guard().push_back(message);
    }
}

#[derive(Debug)]
struct LogViewRow(String);

#[relm4::factory]
impl FactoryComponent for LogViewRow {
    type CommandOutput = ();
    type Init = String;
    type Input = NoMessages;
    type Output = NoMessages;
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        gtk::ListBoxRow {
            set_halign: gtk::Align::Fill,
            set_focusable: false,

            gtk::Label {
                set_halign: gtk::Align::Start,
                set_label: &self.0,
            }
        }
    }

    fn init_model(message: Self::Init, _: &DynamicIndex, _: FactorySender<Self>) -> Self {
        Self(message)
    }
}
