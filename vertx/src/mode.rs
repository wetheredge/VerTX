use embassy_sync::watch;

const SUBS: usize = 1;

#[cfg_attr(not(feature = "configurator"), expect(unused))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Ok,
    #[expect(unused)]
    Armed,
    PreConfigurator,
    Configurator,
    #[expect(unused)]
    Updating,
}

pub(crate) type Watch = watch::Watch<crate::mutex::MultiCore, Mode, SUBS>;
pub(crate) type Receiver = watch::Receiver<'static, crate::mutex::MultiCore, Mode, SUBS>;
