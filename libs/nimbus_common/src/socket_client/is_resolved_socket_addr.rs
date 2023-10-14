use std::net::SocketAddr;

use tokio::net::ToSocketAddrs;
use tokio_socks::IntoTargetAddr;

use crate::{tcp::FramedStream, ResultType};

use super::query_nip_io;

pub trait IsResolvedSocketAddr {
  fn resolve(&self) -> Option<&SocketAddr>;
}

impl IsResolvedSocketAddr for SocketAddr {
  fn resolve(&self) -> Option<&SocketAddr> {
    Some(self)
  }
}

impl IsResolvedSocketAddr for String {
  fn resolve(&self) -> Option<&SocketAddr> {
    None
  }
}

impl IsResolvedSocketAddr for &str {
  fn resolve(&self) -> Option<&SocketAddr> {
    None
  }
}

#[inline]
pub async fn connect_tcp<
  't,
  T: IntoTargetAddr<'t> + ToSocketAddrs + IsResolvedSocketAddr + std::fmt::Display,
>(
  target: T,
  ms_timeout: u64,
) -> ResultType<FramedStream> {
  connect_tcp_local(target, None, ms_timeout).await
}

pub async fn connect_tcp_local<
  't,
  T: IntoTargetAddr<'t> + ToSocketAddrs + IsResolvedSocketAddr + std::fmt::Display,
>(
  target: T,
  local: Option<SocketAddr>,
  ms_timeout: u64,
) -> ResultType<FramedStream> {
  // TODO: get socks from config

  if let Some(target) = target.resolve() {
    if let Some(local) = local {
      if local.is_ipv6() && target.is_ipv4() {
        let target = query_nip_io(target).await?;
        return FramedStream::new(target, Some(local), ms_timeout).await;
      }
    }
  }
  FramedStream::new(target, local, ms_timeout).await
}
