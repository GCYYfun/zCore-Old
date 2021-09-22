pub mod e1000;
pub use e1000::*;

pub use super::*;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use smoltcp::phy::DeviceCapabilities;
use smoltcp::socket::SocketSet;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address};
use smoltcp::Result;
use spin::Mutex;

pub trait NetDriver: Driver {
    // if interrupt belongs to this driver, handle it and return true
    // return false otherwise
    // irq number is provided when available
    // driver should skip handling when irq number is mismatched
    fn try_handle_interrupt(&self, irq: Option<usize>, socketset: &Mutex<SocketSet>) -> bool;

    // get mac address for this device
    fn get_mac(&self) -> EthernetAddress {
        unimplemented!("not a net driver")
    }

    // get interface name for this device
    fn get_ifname(&self) -> String {
        unimplemented!("not a net driver")
    }

    // get ip addresses
    fn get_ip_addresses(&self) -> Vec<IpCidr> {
        unimplemented!("not a net driver")
    }

    // get ipv4 address
    fn ipv4_address(&self) -> Option<Ipv4Address> {
        unimplemented!("not a net driver")
    }

    // manually trigger a poll, use it after sending packets
    fn poll(&self, _socketset: &Mutex<SocketSet>) -> Result<bool> {
        unimplemented!("not a net driver")
    }

    // send an ethernet frame, only use it when necessary
    fn send(&self, _data: &[u8]) -> Option<usize> {
        unimplemented!("not a net driver")
    }

    // get mac address from ip address in arp table
    fn get_arp(&self, _ip: IpAddress) -> Option<EthernetAddress> {
        unimplemented!("not a net driver")
    }

    fn get_device_cap(&self) -> DeviceCapabilities {
        unimplemented!("not a net driver")
    }
}
use downcast_rs::impl_downcast;
impl_downcast!(sync NetDriver);

#[linkage = "weak"]
#[export_name = "hal_get_net_sockets"]
pub fn get_net_sockets() -> Arc<Mutex<SocketSet<'static>>> {
    unimplemented!()
}
