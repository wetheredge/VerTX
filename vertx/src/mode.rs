use embassy_sync::watch;

const SUBS: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Ok,
    #[allow(unused)]
    Armed,
    PreConfigurator,
    Configurator,
    #[allow(unused)]
    Updating,
}

pub type Watch = watch::Watch<crate::mutex::MultiCore, Mode, SUBS>;
pub type Receiver = watch::Receiver<'static, crate::mutex::MultiCore, Mode, SUBS>;
