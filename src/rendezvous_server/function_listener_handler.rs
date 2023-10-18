use std::net::SocketAddr;

use nimbus_common::{
  bytes_codec::BytesCodec,
  futures::StreamExt,
  logger::*,
  timeout,
  tokio::{self, net::TcpStream},
  tokio_util::codec::Framed,
  ResultType,
};

use super::{RendezvousServer, Sink};

impl RendezvousServer {
  pub(super) async fn handle_function_listener(
    &self,
    stream: TcpStream,
    addr: SocketAddr,
    key: &str,
    is_websocket: bool,
  ) {
    debug!(
      "Tcp connection form {:?}, is_websocket: {}",
      addr, is_websocket
    );
    let mut rs = self.clone();
    tokio::spawn(async move {
      // handle listener inner
    });
  }

  #[inline]
  async fn handle_function_listener_inner(
    &mut self,
    stream: TcpStream,
    addr: SocketAddr,
    key: &str,
    is_websocket: bool,
  ) -> ResultType<()> {
    let mut sink;
    if is_websocket {
      let ws_stream = tokio_tungstenite::accept_async(stream).await?;
      let (split_sink, mut split_stream) = ws_stream.split();
      sink = Some(Sink::Ws(split_sink));
      while let Ok(Some(Ok(msg))) = timeout(30_000, split_stream.next()).await {
        // handle tcp
      }
    } else {
      let (split_sink, mut split_stream) =
        Framed::new(stream, BytesCodec::new()).split();
      sink = Some(Sink::TcpStream(split_sink));
      while let Ok(Some(Ok(bytes))) = timeout(30_000, split_stream.next()).await
      {
        // handle tcp
      }
    }

    if sink.is_none() {
      // tcp punch
    }
    Ok(())
  }
}
