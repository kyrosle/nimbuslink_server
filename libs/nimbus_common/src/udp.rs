use socket2::{Domain, Socket, Type};
use std::net::SocketAddr;

pub mod framed_stream;
pub use framed_stream::FramedSocket;

use crate::logger::*;


fn new_socket(
  addr: SocketAddr,
  reuse: bool,
  buf_size: usize,
) -> Result<Socket, std::io::Error> {
  let socket = match addr {
    SocketAddr::V4(..) => Socket::new(Domain::IPV4, Type::DGRAM, None),
    SocketAddr::V6(..) => Socket::new(Domain::IPV6, Type::DGRAM, None),
  }?;

  if reuse {
    #[cfg(unix)]
    socket.set_reuse_port(true)?;
    socket.set_reuse_address(true)?;
  }
  // only non-blocking work with tokio, https://stackoverflow.com/questions/64649405/receiver-on-tokiompscchannel-only-receives-messages-when-buffer-is-full
  socket.set_nonblocking(true)?;
  if buf_size > 0 {
    socket.set_recv_buffer_size(buf_size).ok();
  }
  info!(
    "Receive buf size of udp {} - {:?}",
    addr,
    socket.recv_buffer_size()
  );

  if addr.is_ipv6() && addr.ip().is_unspecified() && addr.port() > 0 {
    socket.set_only_v6(false).ok();
  }
  socket.bind(&addr.into())?;

  Ok(socket)
}
