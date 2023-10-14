use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use anyhow::Context;
use tokio::net::{lookup_host, TcpListener, TcpSocket, ToSocketAddrs};

pub mod dyn_tcp_stream;
pub use dyn_tcp_stream::{DynTcpStream, TcpStreamTrait};
pub mod framed_stream;
pub use framed_stream::FramedStream;
pub mod encrypt;
pub use encrypt::Encrypt;

use crate::ResultType;

/// Create a new Tcp socket.
///
/// - addr: the address to bind the socket to.
/// - reuse: indicates whether the socket should be reuseable.
///
/// The socket is created based on the address type (v4 or v6).
///
/// Finally, the socket is bound to the specified address.
///
fn new_socket(
  addr: std::net::SocketAddr,
  reuse: bool,
) -> Result<TcpSocket, std::io::Error> {
  let socket = match addr {
    std::net::SocketAddr::V4(..) => TcpSocket::new_v4()?,
    std::net::SocketAddr::V6(..) => TcpSocket::new_v6()?,
  };
  if reuse {
    #[cfg(unix)]
    socket.set_reuseport(true).ok();
    socket.set_reuseaddr(true).ok();
  }
  socket.bind(addr)?;
  Ok(socket)
}

/// In the context of Tcp listeners, 'backlog' refers to the maximum number fo
/// pending connections that the listener can have in its queue.
///
/// When a client tries to connect to the listener, but the listener is already
/// handling the maximum number of connections, the additional connection requests
/// are placed in a backlog queue.
///
/// The backlog specifies the size of this queue.
///
/// If the backlog is full, new connection requests will be rejected until
/// there is space in the backlog queue.
const DEFAULT_BACKLOG: u32 = 128;

/// Create a new Tcp listener.
///
/// - addr: the address to bind the socket to.
/// - reuse: indicates whether the socket should be reuseable.
///       If false, it bind the listener to the provided address and return it.
///       Otherwise, it looks up the host address, create a new socket, listen on it with a default backlog.
pub async fn new_listener<T: ToSocketAddrs>(
  addr: T,
  reuse: bool,
) -> ResultType<TcpListener> {
  if !reuse {
    Ok(TcpListener::bind(addr).await?)
  } else {
    let addr = lookup_host(&addr)
      .await?
      .next()
      .context("could not resolve to any address")?;

    new_socket(addr, true)?
      .listen(DEFAULT_BACKLOG)
      .map_err(anyhow::Error::msg)
  }
}

/// Listens for incoming Tcp connections on any ip address and a specified port number.
pub async fn listen_any(port: u16) -> ResultType<TcpListener> {
  // attempts to create a Tcp socket using ipv6.
  //
  // if successful, it sets some socket options for reuse and
  // converts it to a raw file descriptor.
  //  and then creates a new socket `socket2` crate and converts it back to a Tcp socket.
  if let Ok(mut socket) = TcpSocket::new_v6() {
    #[cfg(unix)]
    {
      socket.set_reuseport(true).ok();
      socket.set_reuseaddr(true).ok();
      use std::os::unix::io::{FromRawFd, IntoRawFd};
      let raw_fd = socket.into_raw_fd();
      let sock2 = unsafe { socket2::Socket::from_raw_fd(raw_fd) };
      socket = unsafe { TcpSocket::from_raw_fd(sock2.into_raw_fd()) };
    }

    if socket
      .bind(SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port))
      .is_ok()
    {
      if let Ok(l) = socket.listen(DEFAULT_BACKLOG) {
        return Ok(l);
      }
    }
  }

  // otherwise, creates a new socket using ipv4 and specified port,
  // and listening for incoming connections with a default backlog.
  Ok(
    new_socket(
      SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port),
      true,
    )?
    .listen(DEFAULT_BACKLOG)?,
  )
}
