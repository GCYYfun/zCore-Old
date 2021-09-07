pub mod e1000;
pub use e1000::*;

use alloc::sync::Arc;
use alloc::vec;
use smoltcp::socket::SocketSet;
use spin::Mutex;

lazy_static! {
    /// Global SocketSet in smoltcp.
    ///
    /// Because smoltcp is a single thread network stack,
    /// every socket operation needs to lock this.
    pub static ref SOCKETS: Arc<Mutex<SocketSet<'static>>> =
        Arc::new(Mutex::new(SocketSet::new(vec![])));
}

#[export_name = "hal_get_net_sockets"]
pub fn get_net_sockets() -> Arc<Mutex<SocketSet<'static>>> {
    SOCKETS.clone()
}
