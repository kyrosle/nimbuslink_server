use std::net::SocketAddr;

use nimbus_common::{
  allow_err,
  bytes_codec::BytesCodec,
  futures::StreamExt,
  logger::*,
  timeout,
  tokio::{self, net::TcpStream},
  tokio_util::codec::Framed,
  ResultType,
};

use crate::rendezvous_server::Sink;

use super::RendezvousServer;

impl RendezvousServer {
  pub(super) async fn handle_port_listener(
    &self,
    stream: TcpStream,
    addr: SocketAddr,
    key: &str,
  ) {
    debug!("Tcp connection from {:?}, port listener", addr);
    let mut rs = self.clone();
    let key = key.to_owned();
    tokio::spawn(async move {
      allow_err!(rs.handle_port_listener_inner(stream, addr, &key).await)
    });
  }

  async fn handle_port_listener_inner(
    &mut self,
    stream: TcpStream,
    addr: SocketAddr,
    key: &str,
  ) -> ResultType<()> {
    let (split_sink, mut split_stream) =
      Framed::new(stream, BytesCodec::new()).split();

    let mut sink = Some(Sink::TcpStream(split_sink));

    while let Ok(Some(Ok(bytes))) = timeout(30_000, split_stream.next()).await {
      if !self.handle_tcp(&bytes, &mut sink, addr, key, false).await {
        break;
      }
    }
    if sink.is_none() {}

    debug!("Tcp connection from {:?} closed, port listener", addr);

    Ok(())
  }
}
