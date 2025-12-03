use crate::osdep::mem::PSRAM_ALLOCATOR;
use crate::osdep::net::*;
use alloc::boxed::Box;
use core::ffi::CStr;
use core::fmt::{Debug, Display};
use core::net::SocketAddr;
use edge_http::io::Error;
use edge_nal::{Close, Readable, TcpConnect, TcpShutdown, TcpSplit};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embedded_io::{ErrorKind, ErrorType};
use embedded_io_async::{Read, Write};

static MUTEX: embassy_sync::mutex::Mutex<CriticalSectionRawMutex, bool> =
    embassy_sync::mutex::Mutex::new(false);

pub enum TcpWrapper<'a> {
    Plain(&'a TcpStack),
}

impl<'a> TcpWrapper<'a> {
    pub fn plain(stack: &'a TcpStack) -> Self {
        TcpWrapper::Plain(stack)
    }
}

pub enum TcpSock<'a> {
    Plain(<TcpStack as TcpConnect>::Socket<'a>),
}

impl<'a> TcpSock<'a> {
    pub fn into_inner(self) -> <TcpStack as TcpConnect>::Socket<'a> {
        match self {
            TcpSock::Plain(sock) => sock,
        }
    }
}
impl core::error::Error for EdgeHttpError {}
impl<'a> ErrorType for TcpWrapper<'a> {
    type Error = EdgeHttpError;
}

impl ErrorType for TcpSock<'_> {
    type Error = EdgeHttpError;
}

macro_rules! samesies {
    ($self:ident, $st:ident) => {
        match $self {
            TcpSock::Plain(sock) => Box::pin_in(sock.$st(), &PSRAM_ALLOCATOR)
                .await
                .map_err(EdgeHttpError::from),
        }
    };
}
macro_rules! samesies_arg {
    ($self:ident, $st:ident, $arg:expr) => {
        match $self {
            TcpSock::Plain(sock) => Box::pin(sock.$st($arg)).await.map_err(EdgeHttpError::from),
        }
    };
}

impl TcpShutdown for TcpSock<'_> {
    async fn close(&mut self, what: Close) -> Result<(), Self::Error> {
        // let _ = MUTEX.lock().await;
        match self {
            TcpSock::Plain(plain) => Box::pin_in(plain.close(what), &PSRAM_ALLOCATOR)
                .await
                .map_err(From::from),
        }
    }

    async fn abort(&mut self) -> Result<(), Self::Error> {
        // let _ = MUTEX.lock().await;
        samesies!(self, abort)
    }
}

#[cfg(not(any(all(target_arch = "xtensa", target_os = "none"), feature = "tracing")))]
pub mod split {
    use crate::netclients::edgenal_tls::{EdgeHttpError, TcpSock};
    use crate::osdep::net::TcpSocket;
    use edge_nal::{Readable, TcpSplit};
    use embedded_io::ErrorType;
    use embedded_io_async::{Read, Write};

    pub struct TcpReaderWrapper<'a>(pub(crate) &'a TcpSocket);
    pub struct TcpWriterWrapper<'a>(pub(crate) &'a TcpSocket);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
pub mod split {

    use edge_nal_embassy::{TcpSocketRead, TcpSocketWrite};

    pub struct TcpReaderWrapper<'a>(pub(crate) TcpSocketRead<'a>);
    pub struct TcpWriterWrapper<'a>(pub(crate) TcpSocketWrite<'a>);
}
pub use split::*;

impl ErrorType for TcpReaderWrapper<'_> {
    type Error = EdgeHttpError;
}

impl Read for TcpReaderWrapper<'_> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        // let _ = MUTEX.lock().await;
        self.0.read(buf).await.map_err(EdgeHttpError::from)
    }
}

impl Readable for TcpReaderWrapper<'_> {
    async fn readable(&mut self) -> Result<(), Self::Error> {
        // let _ = MUTEX.lock().await;
        self.0.readable().await.map_err(EdgeHttpError::from)
    }
}

impl ErrorType for TcpWriterWrapper<'_> {
    type Error = EdgeHttpError;
}

impl Write for TcpWriterWrapper<'_> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        // let _ = MUTEX.lock().await;
        self.0.write(buf).await.map_err(EdgeHttpError::from)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        // let _ = MUTEX.lock().await;
        self.0.flush().await.map_err(EdgeHttpError::from)
    }
}

impl TcpSplit for TcpSock<'_> {
    type Read<'a>
        = TcpReaderWrapper<'a>
    where
        Self: 'a;
    type Write<'a>
        = TcpWriterWrapper<'a>
    where
        Self: 'a;

    fn split(&mut self) -> (Self::Read<'_>, Self::Write<'_>) {
        match self {
            TcpSock::Plain(p) => {
                let (read, write) = p.split();

                (TcpReaderWrapper(read), TcpWriterWrapper(write))
            }
        }
    }
}
impl Readable for TcpSock<'_> {
    async fn readable(&mut self) -> Result<(), Self::Error> {
        // let _ = MUTEX.lock().await;
        samesies!(self, readable)
    }
}

impl Write for TcpSock<'_> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        // let _ = MUTEX.lock().await;
        samesies_arg!(self, write, buf)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        // let _ = MUTEX.lock().await;
        samesies!(self, flush)
    }
}

impl Read for TcpSock<'_> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        // let _ = MUTEX.lock().await;
        samesies_arg!(self, read, buf)
    }
}

impl TcpConnect for TcpWrapper<'_> {
    type Error = EdgeHttpError;
    type Socket<'a>
        = TcpSock<'a>
    where
        Self: 'a;

    async fn connect(&self, remote: SocketAddr) -> Result<Self::Socket<'_>, Self::Error> {
        // let _ = MUTEX.lock().await;
        match self {
            TcpWrapper::Plain(plain) => {
                let sock = Box::pin_in(plain.connect(remote), &PSRAM_ALLOCATOR)
                    .await
                    .map_err(EdgeHttpError::from)?;
                Ok(TcpSock::Plain(sock))
            }
        }
    }
}

#[derive(Debug)]
pub enum EdgeHttpError {
    Tcp(TcpError),
}

impl Display for EdgeHttpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <EdgeHttpError as Debug>::fmt(self, f)
    }
}

impl EdgeHttpError {
    pub fn from_tcp_edge(e: Error<TcpError>) -> Option<Self> {
        if let Error::Io(e) = e {
            Some(Self::Tcp(e))
        } else {
            None
        }
    }
    pub fn from_tcp(e: TcpError) -> Self {
        Self::Tcp(e)
    }
}

impl From<Error<TcpError>> for EdgeHttpError {
    fn from(e: Error<TcpError>) -> Self {
        EdgeHttpError::from_tcp_edge(e).unwrap()
    }
}
impl From<TcpError> for EdgeHttpError {
    fn from(e: TcpError) -> Self {
        Self::Tcp(e)
    }
}

impl embedded_io::Error for EdgeHttpError {
    fn kind(&self) -> ErrorKind {
        match self {
            EdgeHttpError::Tcp(tcp) => tcp.kind().into(),
        }
    }
}
