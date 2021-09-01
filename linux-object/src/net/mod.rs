//! Linux socket objects
//!

/// missing documentation
pub mod socket_address;
pub use socket_address::*;

/// missing documentation
pub mod tcp;
pub use tcp::*;

/// missing documentation
pub mod udp;
pub use udp::*;

/// missing documentation
pub mod raw;
pub use raw::*;

// /// missing documentation
// pub mod icmp;
// pub use icmp::*;

// pub mod stack;

// ============= Socket Set =============

use smoltcp::socket::SocketSet;

//lazy
use lazy_static::lazy_static;

// spin
use spin::Mutex;

use alloc::vec;

lazy_static! {
    /// Global SocketSet in smoltcp.
    ///
    /// Because smoltcp is a single thread network stack,
    /// every socket operation needs to lock this.
    pub static ref SOCKETS: Mutex<SocketSet<'static>> =
        Mutex::new(SocketSet::new(vec![]));
}

// ============= Socket Set =============

// ============= Define =============

// ========TCP

/// missing documentation
pub const TCP_SENDBUF: usize = 512 * 1024; // 512K
/// missing documentation
pub const TCP_RECVBUF: usize = 512 * 1024; // 512K

// ========UDP

/// missing documentation
pub const UDP_METADATA_BUF: usize = 1024;
/// missing documentation
pub const UDP_SENDBUF: usize = 64 * 1024; // 64K
/// missing documentation
pub const UDP_RECVBUF: usize = 64 * 1024; // 64K

// ========RAW

/// missing documentation
pub const RAW_METADATA_BUF: usize = 1024;
/// missing documentation
pub const RAW_SENDBUF: usize = 64 * 1024; // 64K
/// missing documentation
pub const RAW_RECVBUF: usize = 64 * 1024; // 64K

// ========RAW

/// missing documentation
pub const ICMP_METADATA_BUF: usize = 1024;
/// missing documentation
pub const ICMP_SENDBUF: usize = 64 * 1024; // 64K
/// missing documentation
pub const ICMP_RECVBUF: usize = 64 * 1024; // 64K

// ========Other

/// missing documentation
pub const IPPROTO_IP: usize = 0;
/// missing documentation
pub const IP_HDRINCL: usize = 3;

// ============= Define =============

// ============= SocketHandle =============

use smoltcp::socket::SocketHandle;

/// A wrapper for `SocketHandle`.
/// Auto increase and decrease reference count on Clone and Drop.
#[derive(Debug)]
struct GlobalSocketHandle(SocketHandle);

impl Clone for GlobalSocketHandle {
    fn clone(&self) -> Self {
        SOCKETS.lock().retain(self.0);
        Self(self.0)
    }
}

impl Drop for GlobalSocketHandle {
    fn drop(&mut self) {
        let mut sockets = SOCKETS.lock();
        sockets.release(self.0);
        sockets.prune();

        // send FIN immediately when applicable
        drop(sockets);
        // poll_ifaces_e1000();
        poll_ifaces_loopback();
    }
}

use kernel_hal::get_net_driver;

//  Safety: call this without SOCKETS locked
// fn poll_ifaces_e1000() {
//     for iface in get_net_driver().iter() {
//         iface.poll(&(*SOCKETS));
//     }
// }

use net_stack::{NetStack, NET_STACK};
// use alloc::vec::Vec;
use hashbrown::HashMap;
use kernel_hal::timer_now;
use smoltcp::time::Instant;
/// miss doc
pub fn get_net_stack() -> HashMap<usize, Arc<dyn NetStack>> {
    NET_STACK.read().clone()
}

/// miss doc
fn poll_ifaces_loopback() {
    for (_key, stack) in get_net_stack().iter() {
        let timestamp = Instant::from_millis(timer_now().as_millis() as i64);
        stack.poll(&(*SOCKETS), timestamp);
    }
}

// ============= SocketHandle =============

// ============= Endpoint =============

use smoltcp::wire::IpEndpoint;

/// missing documentation
#[derive(Clone, Debug)]
pub enum Endpoint {
    /// missing documentation
    Ip(IpEndpoint),
    /// missing documentation
    LinkLevel(LinkLevelEndpoint),
    /// missing documentation
    Netlink(NetlinkEndpoint),
}

/// missing documentation
#[derive(Clone, Debug)]
pub struct LinkLevelEndpoint {
    /// missing documentation
    pub interface_index: usize,
}

impl LinkLevelEndpoint {
    /// missing documentation
    pub fn new(ifindex: usize) -> Self {
        LinkLevelEndpoint {
            interface_index: ifindex,
        }
    }
}

/// missing documentation
#[derive(Clone, Debug)]
pub struct NetlinkEndpoint {
    /// missing documentation
    pub port_id: u32,
    /// missing documentation
    pub multicast_groups_mask: u32,
}

impl NetlinkEndpoint {
    /// missing documentation
    pub fn new(port_id: u32, multicast_groups_mask: u32) -> Self {
        NetlinkEndpoint {
            port_id,
            multicast_groups_mask,
        }
    }
}

// ============= Endpoint =============

// ============= Rand Port =============

// rand
use kernel_hal::rand;

#[allow(unsafe_code)]
/// missing documentation
fn get_ephemeral_port() -> u16 {
    // TODO selects non-conflict high port
    static mut EPHEMERAL_PORT: u16 = 0;
    unsafe {
        if EPHEMERAL_PORT == 0 {
            EPHEMERAL_PORT = (49152 + rand() % (65536 - 49152)) as u16;
        }
        if EPHEMERAL_PORT == 65535 {
            EPHEMERAL_PORT = 49152;
        } else {
            EPHEMERAL_PORT += 1;
        }
        EPHEMERAL_PORT
    }
}

// ============= Rand Port =============

// ============= Util =============

#[allow(unsafe_code)]
/// # Safety
/// Convert C string to Rust string
pub unsafe fn from_cstr(s: *const u8) -> &'static str {
    use core::{slice, str};
    let len = (0usize..).find(|&i| *s.add(i) == 0).unwrap();
    str::from_utf8(slice::from_raw_parts(s, len)).unwrap()
}

// ============= Util =============

use crate::error::*;
use alloc::boxed::Box;
use alloc::fmt::Debug;
use alloc::sync::Arc;
use async_trait::async_trait;
// use core::ops::{Deref, DerefMut};
/// Common methods that a socket must have
#[async_trait]
pub trait Socket: Send + Sync + Debug {
    /// missing documentation
    async fn read(&self, data: &mut [u8]) -> (SysResult, Endpoint);
    /// missing documentation
    fn write(&self, data: &[u8], sendto_endpoint: Option<Endpoint>) -> SysResult;
    /// missing documentation
    fn poll(&self) -> (bool, bool, bool); // (in, out, err)
    /// missing documentation
    async fn connect(&self, endpoint: Endpoint) -> SysResult;
    /// missing documentation
    fn bind(&mut self, _endpoint: Endpoint) -> SysResult {
        Err(LxError::EINVAL)
    }
    /// missing documentation
    fn listen(&mut self) -> SysResult {
        Err(LxError::EINVAL)
    }
    /// missing documentation
    fn shutdown(&self) -> SysResult {
        Err(LxError::EINVAL)
    }
    /// missing documentation
    async fn accept(&mut self) -> LxResult<(Arc<Mutex<dyn Socket>>, Endpoint)> {
        Err(LxError::EINVAL)
    }
    /// missing documentation
    fn endpoint(&self) -> Option<Endpoint> {
        None
    }
    /// missing documentation
    fn remote_endpoint(&self) -> Option<Endpoint> {
        None
    }
    /// missing documentation
    fn setsockopt(&self, _level: usize, _opt: usize, _data: &[u8]) -> SysResult {
        warn!("setsockopt is unimplemented");
        Ok(0)
    }
    /// missing documentation
    fn ioctl(&self, _request: usize, _arg1: usize, _arg2: usize, _arg3: usize) -> SysResult {
        warn!("ioctl is unimplemented for this socket");
        Ok(0)
    }
    /// missing documentation
    fn fcntl(&self, _cmd: usize, _arg: usize) -> SysResult {
        warn!("ioctl is unimplemented for this socket");
        Ok(0)
    }
}