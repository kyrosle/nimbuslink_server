use std::{
  net::SocketAddr,
  ops::{Deref, DerefMut},
};

use anyhow::bail;
use bytes::{Bytes, BytesMut};
use futures_util::{SinkExt, StreamExt};
use protobuf::Message;
use sodiumoxide::crypto::secretbox::{self, Key, Nonce};
use tokio::net::{lookup_host, ToSocketAddrs};
use tokio_socks::{tcp::Socks5Stream, IntoTargetAddr, ToProxyAddrs};
use tokio_util::codec::Framed;

use crate::{bytes_codec::BytesCodec, ResultType};

use super::{DynTcpStream, Encrypt, TcpStreamTrait};

/// Used to handle Tcp communication.
///
/// by using `DynTcpSteam` trait object, it can work with any type
/// implements the `TcpStreamTrait` trait, without needing to know the concrete type at compline time.
pub struct FramedStream {
  framed: Framed<DynTcpStream, BytesCodec>,
  local_addr: SocketAddr,
  encrypt: Option<Encrypt>,
  send_timeout: u64,
}

impl Deref for FramedStream {
  type Target = Framed<DynTcpStream, BytesCodec>;
  fn deref(&self) -> &Self::Target {
    &self.framed
  }
}

impl DerefMut for FramedStream {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.framed
  }
}

impl FramedStream {
  pub async fn new<T: ToSocketAddrs + std::fmt::Display>(
    remote_addr: T,
    local_addr: Option<SocketAddr>,
    ms_timeout: u64,
  ) -> ResultType<Self> {
    // perform DNS lookup
    for remote_addr in lookup_host(&remote_addr).await? {
      let local = if let Some(addr) = local_addr {
        addr
      } else {
        // from config
        unimplemented!();
      };

      if let Ok(socket) = super::new_socket(local, true) {
        if let Ok(Ok(stream)) =
          crate::timeout(ms_timeout, socket.connect(remote_addr)).await
        {
          stream.set_nodelay(true).ok();
          let addr = stream.local_addr()?;
          return Ok(FramedStream {
            framed: Framed::new(
              DynTcpStream::from_stream(Box::new(stream)),
              BytesCodec::new(),
            ),
            local_addr: addr,
            encrypt: None,
            send_timeout: 0,
          });
        }
      }
    }
    bail!(format!("Failed to connect to {remote_addr}"));
  }

  /// Connect the target address, if the proxy is set, the connection using the proxy, otherwise throw an Error.
  pub async fn connect<'a, 't, P, T>(
    proxy: P,
    target: T,
    local_addr: Option<SocketAddr>,
    username: &'a str,
    password: &'a str,
    ms_timeout: u64,
  ) -> ResultType<Self>
  where
    P: ToProxyAddrs,
    T: IntoTargetAddr<'t>,
  {
    // first checks if the provided proxy can be resolved to an address.
    // if it can establish a connection using the proxy.
    if let Some(Ok(proxy)) = proxy.to_proxy_addrs().next().await {
      let local = if let Some(addr) = local_addr {
        addr
      } else {
        // form config
        unimplemented!();
      };

      let stream = crate::timeout(
        ms_timeout,
        super::new_socket(local, true)?.connect(proxy),
      )
      .await??;
      stream.set_nodelay(true).ok();

      // check the username is provided
      let stream = if username.trim().is_empty() {
        crate::timeout(
          ms_timeout,
          Socks5Stream::connect_with_socket(stream, target),
        )
        .await??
      } else {
        // establish a connection using the authentication.
        crate::timeout(
          ms_timeout,
          Socks5Stream::connect_with_password_and_socket(
            stream, target, username, password,
          ),
        )
        .await??
      };

      let addr = stream.local_addr()?;

      return Ok(FramedStream {
        framed: Framed::new(
          DynTcpStream::from_stream(Box::new(stream)),
          BytesCodec::new(),
        ),
        local_addr: addr,
        encrypt: None,
        send_timeout: 0,
      });
    }
    bail!("could not resolve to any address");
  }

  pub fn local_addr(&self) -> SocketAddr {
    self.local_addr
  }

  pub fn set_send_timeout(&mut self, ms: u64) {
    self.send_timeout = ms;
  }

  pub fn from(
    stream: impl TcpStreamTrait + Send + Sync + 'static,
    addr: SocketAddr,
  ) -> Self {
    FramedStream {
      framed: Framed::new(
        DynTcpStream::from_stream(Box::new(stream)),
        BytesCodec::new(),
      ),
      local_addr: addr,
      encrypt: None,
      send_timeout: 0,
    }
  }

  pub fn set_raw(&mut self) {
    self.framed.codec_mut().set_raw();
    self.encrypt = None;
  }

  pub fn is_secured(&self) -> bool {
    self.encrypt.is_some()
  }

  pub fn set_key(&mut self, key: Key) {
    self.encrypt = Some(Encrypt::new(key));
  }

  /// seq_num: cryptographic protocols to provide uniqueness and integrity for messages
  pub fn get_nonce(seq_num: u64) -> Nonce {
    let mut nonce = Nonce([0u8; secretbox::NONCEBYTES]);
    nonce.0[..std::mem::size_of_val(&seq_num)]
      .copy_from_slice(&seq_num.to_le_bytes());
    nonce
  }

  #[inline]
  pub async fn next_timeout(
    &mut self,
    ms: u64,
  ) -> Option<Result<BytesMut, std::io::Error>> {
    if let Ok(res) = crate::timeout(ms, self.next()).await {
      res
    } else {
      None
    }
  }

  #[inline]
  pub async fn send(&mut self, msg: &impl Message) -> ResultType<()> {
    self.send_raw(msg.write_to_bytes()?).await
  }

  #[inline]
  pub async fn send_raw(&mut self, msg: Vec<u8>) -> ResultType<()> {
    let mut msg = msg;
    if let Some(key) = self.encrypt.as_mut() {
      msg = key.encrypt(&msg);
    }
    self.send_bytes(bytes::Bytes::from(msg)).await?;
    Ok(())
  }

  #[inline]
  pub async fn send_bytes(&mut self, bytes: Bytes) -> ResultType<()> {
    if self.send_timeout > 0 {
      crate::timeout(self.send_timeout, self.framed.send(bytes)).await??;
    } else {
      self.framed.send(bytes).await?;
    }
    Ok(())
  }
}
