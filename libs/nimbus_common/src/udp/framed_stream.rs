use std::net::SocketAddr;

use anyhow::{anyhow, Context};
use bytes::{Bytes, BytesMut};
use futures_util::{SinkExt, StreamExt};
use tokio::net::{lookup_host, ToSocketAddrs, UdpSocket};
use tokio_socks::{
  udp::Socks5UdpFramed, IntoTargetAddr, TargetAddr, ToProxyAddrs,
};
use tokio_util::{codec::BytesCodec, udp::UdpFramed};

use crate::{logger::*, ResultType};

use super::new_socket;

pub enum FramedSocket {
  Direct(UdpFramed<BytesCodec>),
  ProxySocks(Socks5UdpFramed),
}

impl FramedSocket {
  pub async fn new<T: ToSocketAddrs>(
    addr: T,
    reuse: bool,
    buf_size: usize,
  ) -> ResultType<Self> {
    let addr = lookup_host(&addr)
      .await?
      .next()
      .context("could not resolve to any address")?;

    Ok(FramedSocket::Direct(UdpFramed::new(
      UdpSocket::from_std(new_socket(addr, reuse, buf_size)?.into())?,
      BytesCodec::new(),
    )))
  }

  pub async fn new_proxy<'a, P: ToProxyAddrs, T: ToSocketAddrs>(
    proxy: P,
    local: T,
    username: &'a str,
    password: &'a str,
    ms_timeout: u64,
  ) -> ResultType<Self> {
    let framed = if username.trim().is_empty() {
      crate::timeout(ms_timeout, Socks5UdpFramed::connect(proxy, Some(local)))
        .await??
    } else {
      crate::timeout(
        ms_timeout,
        Socks5UdpFramed::connect_with_password(
          proxy,
          Some(local),
          username,
          password,
        ),
      )
      .await??
    };
    trace!(
      "Socks5 udp connected, local addr: {:?}, target addr: {}",
      framed.local_addr(),
      framed.socks_addr()
    );
    Ok(FramedSocket::ProxySocks(framed))
  }

  // https://stackoverflow.com/a/68733302/1926020
  #[inline]
  pub async fn send_raw(
    &mut self,
    msg: &'static [u8],
    addr: impl IntoTargetAddr<'static>,
  ) -> ResultType<()> {
    let addr = addr.into_target_addr()?.to_owned();

    match self {
      FramedSocket::Direct(framed_socket) => {
        if let TargetAddr::Ip(addr) = addr {
          framed_socket.send((Bytes::from(msg), addr)).await?
        }
      }
      FramedSocket::ProxySocks(framed_socket) => {
        framed_socket.send((Bytes::from(msg), addr)).await?
      }
    };

    Ok(())
  }

  #[inline]
  pub async fn next(
    &mut self,
  ) -> Option<ResultType<(BytesMut, TargetAddr<'static>)>> {
    match self {
      FramedSocket::Direct(framed_socket) => match framed_socket.next().await {
        Some(Ok((data, addr))) => {
          Some(Ok((data, addr.into_target_addr().ok()?.to_owned())))
        }
        Some(Err(e)) => Some(Err(anyhow!(e))),
        None => None,
      },
      FramedSocket::ProxySocks(framed_socket) => {
        match framed_socket.next().await {
          Some(Ok((data, _))) => Some(Ok((data.data, data.dst_addr))),
          Some(Err(e)) => Some(Err(anyhow!(e))),
          None => None,
        }
      }
    }
  }

  #[inline]
  pub async fn next_timeout(
    &mut self,
    ms: u64,
  ) -> Option<ResultType<(BytesMut, TargetAddr<'static>)>> {
    if let Ok(res) =
      tokio::time::timeout(std::time::Duration::from_millis(ms), self.next())
        .await
    {
      res
    } else {
      None
    }
  }

  pub fn local_addr(&self) -> Option<SocketAddr> {
    if let FramedSocket::Direct(x) = self {
      if let Ok(v) = x.get_ref().local_addr() {
        return Some(v);
      }
    }
    None
  }
}
