#[cfg_attr(not(all(target_arch = "xtensa")), path = "net_hosted.rs")]
#[cfg_attr(all(target_arch = "xtensa", target_os = "none"), path = "net_esp.rs")]
#[cfg_attr(all(target_arch = "xtensa", target_os = "espidf"), path = "net_idf.rs")]
mod network_inner;
pub mod net {
    pub use super::network_inner::*;
    use crate::osdep::typedefs::GlobalStatics;
    use core::net::SocketAddr;
    use edge_nal::{AddrType, Dns};

    pub async fn get_sockaddr(statics: GlobalStatics, host: &str, port: u16) -> SocketAddr {
        let addr = statics
            .core0_net
            .dns
            .get_host_by_name(host, AddrType::IPv4)
            .await
            .unwrap();
        SocketAddr::new(addr, port)
    }
}
