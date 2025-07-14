#[cfg(feature = "network-usb-ethernet")]
pub(crate) mod ncm_cdc;

use embassy_executor::{Spawner, task};
use static_cell::ConstStaticCell;

const MAX_PACKET_SIZE: u8 = 64;

pub struct Init {
    #[cfg(feature = "network-usb-ethernet")]
    pub network: ncm_cdc::NetDriver,
}

pub fn init(spawner: Spawner, usb: crate::hal::Usb) -> Init {
    let mut config = embassy_usb::Config::new(0xC0DE, 0xCAFE);
    config.manufacturer = Some("VerTX");
    config.max_packet_size_0 = MAX_PACKET_SIZE;

    static CONFIG_DESC: ConstStaticCell<[u8; 256]> = ConstStaticCell::new([0; 256]);
    static BOS_DESC: ConstStaticCell<[u8; 256]> = ConstStaticCell::new([0; 256]);
    static CONTROL_BUF: ConstStaticCell<[u8; 128]> = ConstStaticCell::new([0; 128]);
    let mut builder = embassy_usb::Builder::new(
        usb,
        config,
        CONFIG_DESC.take().as_mut_slice(),
        BOS_DESC.take().as_mut_slice(),
        &mut [],
        CONTROL_BUF.take().as_mut_slice(),
    );

    #[cfg(feature = "network-usb-ethernet")]
    let cdc_ncm = ncm_cdc::init(&mut builder);

    let usb = builder.build();
    spawner.must_spawn(run_usb(usb));

    Init {
        #[cfg(feature = "network-usb-ethernet")]
        network: ncm_cdc::get_network(spawner, cdc_ncm),
    }
}

#[task]
async fn run_usb(mut device: embassy_usb::UsbDevice<'static, crate::hal::Usb>) -> ! {
    device.run().await
}
