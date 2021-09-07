mod bus;
pub use bus::*;

mod net;
pub use net::*;

// alloc
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
// downcast
use downcast_rs::DowncastSync;

#[derive(Debug, Eq, PartialEq)]
pub enum DeviceType {
    Net,
    Gpu,
    Input,
    Block,
    Rtc,
    Serial,
    Intc,
}

pub trait Driver: DowncastSync + Send + Sync {
    // return the correspondent device type, see DeviceType
    fn device_type(&self) -> DeviceType;

    // get unique identifier for this device
    // should be different for each instance
    fn get_id(&self) -> String;

    // trait casting
    fn as_net(&self) -> Option<&dyn NetDriver> {
        None
    }
}

// function

#[linkage = "weak"]
#[export_name = "hal_driver"]
pub fn get_driver() -> Vec<Arc<dyn Driver>> {
    unimplemented!()
}

#[linkage = "weak"]
#[export_name = "hal_get_driver"]
pub fn get_net_driver() -> Vec<Arc<dyn NetDriver>> {
    unimplemented!()
}
