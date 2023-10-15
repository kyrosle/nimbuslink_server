// common module
pub mod bytes_codec;
pub mod socket_client;
pub mod tcp;
pub mod udp;
pub mod compress;
pub mod config;

// extern
pub use bytes;
pub use env_logger;
pub use futures;
pub use log;
pub use tokio;
pub use tokio_util;

// export the logger function
pub mod logger {
  pub use log::{debug, error, info, warn, trace};
}

pub type ResultType<F, E = anyhow::Error> = anyhow::Result<F, E>;

#[inline]
pub fn timeout<T: std::future::Future>(
  ms: u64,
  future: T,
) -> tokio::time::Timeout<T> {
  tokio::time::timeout(std::time::Duration::from_micros(ms), future)
}

const IPV4_REGEX_MATCH: &str = r"^(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)(:\d+)?$";
#[inline]
pub fn is_ipv4_str(id: &str) -> bool {
  if let Ok(reg) = regex::Regex::new(IPV4_REGEX_MATCH) {
    reg.is_match(id)
  } else {
    false
  }
}

const IPV6_REGEX_MATCH: &str = r"^((([a-fA-F0-9]{1,4}:{1,2})+[a-fA-F0-9]{1,4})|(\[([a-fA-F0-9]{1,4}:{1,2})+[a-fA-F0-9]{1,4}\]:\d+))$";
#[inline]
pub fn is_ipv6_str(id: &str) -> bool {
  if let Ok(reg) = regex::Regex::new(IPV6_REGEX_MATCH) {
    reg.is_match(id)
  } else {
    false
  }
}
