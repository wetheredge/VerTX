use embassy_executor::{Spawner, task};
use embassy_usb::class::cdc_ncm::{self, CdcNcmClass, State};
use static_cell::StaticCell;

const MAC_ADDR: [u8; 6] = [0xCC; 6];
const HOST_MAC_ADDR: [u8; 6] = [0x88; 6];
const MTU: usize = 1514;

type Class = CdcNcmClass<'static, crate::hal::Usb>;
pub(crate) type NetDriver = cdc_ncm::embassy_net::Device<'static, MTU>;

pub(super) fn init(builder: &mut embassy_usb::Builder<'static, crate::hal::Usb>) -> Class {
    static STATE: StaticCell<State> = StaticCell::new();
    let state = STATE.init_with(State::new);

    CdcNcmClass::new(builder, state, HOST_MAC_ADDR, super::MAX_PACKET_SIZE.into())
}

pub(super) fn get_network(spawner: Spawner, class: Class) -> NetDriver {
    static STATE: StaticCell<cdc_ncm::embassy_net::State<MTU, 4, 4>> = StaticCell::new();
    let state = STATE.init_with(cdc_ncm::embassy_net::State::new);

    let (runner, device) = class.into_embassy_net_device::<MTU, 4, 4>(state, MAC_ADDR);
    spawner.must_spawn(run_cdc_ncm(runner));

    device
}

#[task]
async fn run_cdc_ncm(runner: cdc_ncm::embassy_net::Runner<'static, crate::hal::Usb, MTU>) -> ! {
    runner.run().await
}
