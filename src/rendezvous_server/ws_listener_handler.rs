use std::net::SocketAddr;

use nimbus_common::{
  futures::StreamExt,
  logger::*,
  timeout,
  tokio::{self, net::TcpStream},
  ResultType, allow_err,
};

use super::{RendezvousServer, Sink};

impl RendezvousServer {
  pub(super) async fn handle_ws_listener(
    &self,
    stream: TcpStream,
    addr: SocketAddr,
    key: &str,
  ) {
    debug!("Tcp connection from {:?}, websocket listener", addr);
    let mut rs = self.clone();
    let key = key.to_owned();
    tokio::spawn(async move {
      allow_err!(rs.handle_ws_listener_inner(stream, addr, &key).await);
    });
  }

  #[inline]
  async fn handle_ws_listener_inner(
    &mut self,
    stream: TcpStream,
    addr: SocketAddr,
    key: &str,
  ) -> ResultType<()> {
    let ws_stream = tokio_tungstenite::accept_async(stream).await?;

    let (split_sink, mut split_stream) = ws_stream.split();

    let mut sink = Some(Sink::Ws(split_sink));
    while let Ok(Some(Ok(msg))) = timeout(30_000, split_stream.next()).await {
      if let tungstenite::Message::Binary(bytes) = msg {
        if !self.handle_tcp(&bytes, &mut sink, addr, key, false).await {
          break;
        }
      }
    }

    if sink.is_none() {
      // TODO: tcp_punch remove addr
    }

    debug!("Tcp connection from {:?} closed, websocket listener", addr);

    Ok(())
  }
}
