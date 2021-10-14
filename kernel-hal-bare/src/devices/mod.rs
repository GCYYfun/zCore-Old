pub mod bus;
pub mod net;
pub use net::*;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;

use lazy_static::lazy_static;

use spin::RwLock;

use pci::Location;

use kernel_hal::Driver;
use kernel_hal::NetDriver;

use log::warn;

lazy_static! {
    pub static ref DRIVERS: RwLock<Vec<Arc<dyn Driver>>> = RwLock::new(Vec::new());
    pub static ref NET_DRIVERS: RwLock<Vec<Arc<dyn NetDriver>>> = RwLock::new(Vec::new());
    pub static ref PCI_DRIVERS: RwLock<BTreeMap<Location, Arc<dyn Driver>>> =
        RwLock::new(BTreeMap::new());
}

pub fn devices_init() {
    bus::pci::init();
}

#[export_name = "hal_get_driver"]
#[no_mangle]
pub extern "C" fn get_net_driver() -> Vec<Arc<dyn NetDriver>> {
    NET_DRIVERS.read().clone()
}

use net_stack::resource::get_net_sockets;
#[no_mangle]
pub extern "C" fn poll_ifaces_e1000() {
    for iface in get_net_driver().iter() {
        if let Ok(b) = iface.poll(&(*get_net_sockets())) {
            warn!("b:{}",b);
        }
    }
}
