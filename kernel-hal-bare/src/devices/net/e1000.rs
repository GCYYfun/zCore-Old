// alloc
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

// smoltcp
use smoltcp::iface::Interface;
use smoltcp::iface::InterfaceBuilder;
use smoltcp::iface::NeighborCache;
use smoltcp::iface::Route;
use smoltcp::iface::Routes;
use smoltcp::phy::Device;
use smoltcp::phy::{self, DeviceCapabilities};
use smoltcp::socket::SocketSet;
use smoltcp::time::Instant;
use smoltcp::wire::EthernetAddress;
use smoltcp::wire::IpAddress;
use smoltcp::wire::IpCidr;
use smoltcp::wire::Ipv4Address;
use smoltcp::Result;

// isomorphic_drivers
use isomorphic_drivers::net::ethernet::intel::e1000::E1000;
use isomorphic_drivers::net::ethernet::structs::EthernetAddress as DriverEthernetAddress;

// ctate
use super::SOCKETS;
use crate::arch::timer_now;
use crate::devices::NET_DRIVERS;
use crate::Box;
use crate::PAGE_SIZE;
use kernel_hal::{DeviceType, Driver, NetDriver};

// spin
use spin::Mutex;

#[derive(Clone)]
pub struct E1000Driver(Arc<Mutex<E1000<Provider>>>);

pub struct E1000RxToken(Vec<u8>);
pub struct E1000TxToken(E1000Driver);

impl phy::Device<'_> for E1000Driver {
    type RxToken = E1000RxToken;
    type TxToken = E1000TxToken;

    fn receive(&mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        self.0
            .lock()
            .receive()
            .map(|vec| (E1000RxToken(vec), E1000TxToken(self.clone())))
    }

    fn transmit(&mut self) -> Option<Self::TxToken> {
        if self.0.lock().can_send() {
            Some(E1000TxToken(self.clone()))
        } else {
            None
        }
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1536;
        caps.max_burst_size = Some(64);
        caps
    }
}

pub struct E1000Interface {
    pub iface: Mutex<Interface<'static, E1000Driver>>,
    driver: E1000Driver,
    name: String,
    irq: Option<usize>,
}

impl Driver for E1000Interface {
    fn device_type(&self) -> DeviceType {
        DeviceType::Net
    }

    fn get_id(&self) -> String {
        String::from("e1000")
    }

    fn as_net(&self) -> Option<&dyn NetDriver> {
        Some(self)
    }
}

impl NetDriver for E1000Interface {
    fn try_handle_interrupt(&self, irq: Option<usize>, socketset: &Mutex<SocketSet>) -> bool {
        warn!("handle interrupt ================================================");
        if irq.is_some() && self.irq.is_some() && irq != self.irq {
            // not ours, skip it
            return false;
        }

        let data = self.driver.0.lock().handle_interrupt();
        warn!("data : {}", data);
        if data {
            let timestamp = Instant::from_millis(timer_now().as_millis() as i64);
            let mut sockets = socketset.lock();
            match self.iface.lock().poll(&mut sockets, timestamp) {
                Ok(b) => {
                    warn!("interrupt iface poll,result : {}", b);
                }
                Err(err) => {
                    debug!("poll got err {}", err);
                }
            }
        }
        return data;
    }

    fn get_mac(&self) -> EthernetAddress {
        self.iface.lock().ethernet_addr()
    }

    fn get_ifname(&self) -> String {
        self.name.clone()
    }

    // get ip addresses
    fn get_ip_addresses(&self) -> Vec<IpCidr> {
        Vec::from(self.iface.lock().ip_addrs())
    }

    fn ipv4_address(&self) -> Option<Ipv4Address> {
        self.iface.lock().ipv4_address()
    }

    fn poll(&self, socketset: &Mutex<SocketSet>) -> Result<bool> {
        let timestamp = Instant::from_millis(timer_now().as_millis() as i64);
        let mut sockets = socketset.lock();
        match self.iface.lock().poll(&mut sockets, timestamp) {
            Ok(b) => Ok(b),
            Err(err) => {
                debug!("poll got err {}", err);
                Err(err)
            }
        }
    }

    fn send(&self, data: &[u8]) -> Option<usize> {
        use smoltcp::phy::TxToken;
        let token = E1000TxToken(self.driver.clone());
        if token
            .consume(Instant::from_millis(0), data.len(), |buffer| {
                buffer.copy_from_slice(&data);
                Ok(())
            })
            .is_ok()
        {
            Some(data.len())
        } else {
            None
        }
    }

    fn get_arp(&self, _ip: IpAddress) -> Option<EthernetAddress> {
        // let iface = self.iface.lock();
        // let cache = iface.neighbor_cache();
        // cache.lookup_pure(&ip, Instant::from_millis(0))
        unimplemented!()
    }

    fn get_device_cap(&self) -> DeviceCapabilities {
        let driver = &self.driver;
        driver.capabilities().clone()
    }
}

impl phy::RxToken for E1000RxToken {
    fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> Result<R>
    where
        F: FnOnce(&mut [u8]) -> Result<R>,
    {
        // warn!("Enter : E1000TxToken");
        f(&mut self.0)
    }
}

impl phy::TxToken for E1000TxToken {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> Result<R>
    where
        F: FnOnce(&mut [u8]) -> Result<R>,
    {
        // warn!("Enter : E1000RxToken");
        let mut buffer = [0u8; PAGE_SIZE];
        let result = f(&mut buffer[..len]);

        let mut driver = (self.0).0.lock();
        driver.send(&buffer);

        result
    }
}

#[export_name = "hal_net_e1000_init"]
pub fn init(name: String, irq: Option<usize>, header: usize, size: usize, index: usize) {
    warn!("Probing e1000 {}, interrupt : {:?}", name, irq);

    // randomly generated
    let mac: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];

    let e1000 = E1000::new(header, size, DriverEthernetAddress::from_bytes(&mac));

    let net_driver = E1000Driver(Arc::new(Mutex::new(e1000)));

    let ethernet_addr = EthernetAddress::from_bytes(&mac);
    let ip_addrs = [IpCidr::new(IpAddress::v4(10, 0, 2, 15), 24)];
    // let ip_addrs = [IpCidr::new(IpAddress::v4(127,0, 0,1), 24)];
    let default_gateway = Ipv4Address::new(10, 0, 2, 2);
    // let default_gateway = Ipv4Address::new(127, 0, 0, 1);
    let neighbor_cache = NeighborCache::new(BTreeMap::new());
    static mut routes_storage: [Option<(IpCidr, Route)>; 1] = [None; 1];
    let mut routes = unsafe { Routes::new(&mut routes_storage[..]) };
    routes.add_default_ipv4_route(default_gateway).unwrap();
    let iface = InterfaceBuilder::new(net_driver.clone())
        .ethernet_addr(ethernet_addr)
        .ip_addrs(ip_addrs)
        .routes(routes)
        .neighbor_cache(neighbor_cache)
        .finalize();

    let e1000_iface = E1000Interface {
        iface: Mutex::new(iface),
        driver: net_driver.clone(),
        name,
        irq,
    };

    let driver = Arc::new(e1000_iface);
    NET_DRIVERS.write().push(driver);

    // use net_stack::NET_STACK;
    // use net_stack::{Stack,StackInner};
    // // use net_stack::NET_STACK;
    // // 创建 一个 栈
    // let stack_inner = StackInner { interface: iface };
    // // stack_inner.interfaces.insert(0, iface);

    // // 把 栈 返回

    // let mut stack = Stack {
    //     inner: Arc::new(Mutex::new(stack_inner)),
    // };

    // // let info = InterfaceInfo::new(String::from("smoltcp"));
    // let mut w = NET_STACK.write();

    // w.lock().insert(1, Arc::new(stack));

    // use crate::arch::interrupt::register_irq_handler;
    // register_irq_handler(57,Box::new(net_interrupt_test));
    use crate::arch::interrupt::IRQ_TABLE;
    IRQ_TABLE
        .lock()
        .insert(57, Some(Box::new(net_interrupt_test)));
}

use net_stack::resource::get_net_sockets;

fn net_interrupt_test() {
    warn!("interrupt");
    for iface in NET_DRIVERS.read().clone().iter() {
        warn!("erver iface net_interrupt iface : {:?}", iface.get_ifname());
        iface.try_handle_interrupt(Some(25), &get_net_sockets());
    }
}

// provider

use crate::drivers::virtio::virtio::virtio_dma_alloc;
// use crate::drivers::virtio::virtio::virtio_dma_dealloc;
#[allow(unused_imports)]
use crate::{
    hal_frame_alloc_contiguous as frame_alloc_contiguous, hal_frame_dealloc as frame_dealloc,
    phys_to_virt, virt_to_phys,
};
use isomorphic_drivers::provider;
pub struct Provider;

impl provider::Provider for Provider {
    const PAGE_SIZE: usize = PAGE_SIZE;

    fn alloc_dma(size: usize) -> (usize, usize) {
        let paddr = virtio_dma_alloc(size / PAGE_SIZE);
        let vaddr = phys_to_virt(paddr);
        (vaddr, paddr)
    }

    fn dealloc_dma(vaddr: usize, size: usize) {
        let paddr = virt_to_phys(vaddr);
        for i in 0..size / PAGE_SIZE {
            unsafe {
                frame_dealloc(&(paddr + i * PAGE_SIZE));
            }
        }
    }
}

// #[no_mangle]
// extern "C" fn virtio_dma_alloc(pages: usize) -> PhysAddr {
//     let paddr = unsafe { alloc_frame_contiguous(pages, 0).unwrap() };
//     trace!("alloc DMA: paddr={:#x}, pages={}", paddr, pages);
//     paddr
// }

// #[no_mangle]
// extern "C" fn virtio_dma_dealloc(paddr: PhysAddr, pages: usize) -> i32 {
//     for i in 0..pages {
//         unsafe {
//             dealloc_frame(&(paddr + i * PAGE_SIZE));
//         }
//     }
//     trace!("dealloc DMA: paddr={:#x}, pages={}", paddr, pages);
//     0
// }

// #[no_mangle]
// extern "C" fn virtio_phys_to_virt(paddr: PhysAddr) -> VirtAddr {
//     phys_to_virt(paddr)
// }

// #[no_mangle]
// extern "C" fn virtio_virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
//     virt_to_phys(vaddr)
// }

type VirtAddr = usize;
type PhysAddr = usize;
