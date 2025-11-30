use edge_nal_embassy::Dns;
use edge_nal_embassy::Tcp;
use esp_rtos::embassy::Executor as EmbassyExecutor;

pub const NUM_CONNECTIONS: usize = 3;
pub const TOTAL_CONNECTIONS: usize = crate::osdep::net::NUM_CONNECTIONS + 1;
const BUF_SIZE: usize = 1024;
pub type Executor = EmbassyExecutor;
pub type TcpStack = Tcp<'static, NUM_CONNECTIONS, BUF_SIZE, BUF_SIZE>;
pub type TcpSocket = edge_nal_embassy::TcpSocket<'static, NUM_CONNECTIONS, BUF_SIZE, BUF_SIZE>;
pub type TcpBuffs = edge_nal_embassy::TcpBuffers<NUM_CONNECTIONS, BUF_SIZE, BUF_SIZE>;
pub use edge_nal_embassy::TcpError;

pub type DnsStack = Dns<'static>;
