
// common module
pub mod bytes_codec;
pub mod tcp;
pub mod udp;

// extern
pub use bytes;
pub use env_logger;
pub use futures;
pub use log;
pub use tokio;
pub use tokio_util;

pub type ResultType<F, E = anyhow::Error> = anyhow::Result<F, E>;

#[inline]
pub fn timeout<T: std::future::Future>(
  ms: u64,
  future: T,
) -> tokio::time::Timeout<T> {
  tokio::time::timeout(std::time::Duration::from_micros(ms), future)
}
