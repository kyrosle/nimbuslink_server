use std::{
  pin::Pin,
  task::{Context, Poll},
};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// Represents a Tcp stream.
pub trait TcpStreamTrait: AsyncRead + AsyncWrite + Unpin {}

impl<R: AsyncRead + AsyncWrite + Unpin> TcpStreamTrait for R {}

/// Represents a Tcp Stream as a dynamic trait object.
pub struct DynTcpStream(Box<dyn TcpStreamTrait + Send + Sync>);

impl DynTcpStream {
  pub fn from_stream(inner: Box<dyn TcpStreamTrait + Send + Sync>) -> Self {
    DynTcpStream(inner)
  }
}

impl Unpin for DynTcpStream {}

impl AsyncRead for DynTcpStream {
  fn poll_read(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &mut ReadBuf<'_>,
  ) -> Poll<std::io::Result<()>> {
    AsyncRead::poll_read(Pin::new(&mut self.0), cx, buf)
  }
}

impl AsyncWrite for DynTcpStream {
  fn poll_write(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &[u8],
  ) -> Poll<Result<usize, std::io::Error>> {
    AsyncWrite::poll_write(Pin::new(&mut self.0), cx, buf)
  }

  fn poll_flush(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Result<(), std::io::Error>> {
    AsyncWrite::poll_flush(Pin::new(&mut self.0), cx)
  }

  fn poll_shutdown(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Result<(), std::io::Error>> {
    AsyncWrite::poll_shutdown(Pin::new(&mut self.0), cx)
  }
}
