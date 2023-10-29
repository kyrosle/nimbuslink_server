use std::net::SocketAddr;

use nimbus_common::{
  protobuf::Message,
  protos::rendezvous::{
    rendezvous_message, RendezvousMessage, TestNatResponse,
  },
  tcp::FramedStream,
  timeout,
  tokio::{
    self,
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
  },
};

use super::RendezvousServer;

impl RendezvousServer {
  pub(super) async fn handle_nat_listener(
    &self,
    stream: TcpStream,
    addr: SocketAddr,
  ) {
    let mut rs = self.clone();
    if addr.ip().is_loopback() {
      tokio::spawn(async move {
        let mut stream = stream;
        let mut buffer = [0; 1024];
        if let Ok(Ok(n)) = timeout(1000, stream.read(&mut buffer[..])).await {
          if let Ok(data) = std::str::from_utf8(&buffer[..n]) {
            let res = rs.check_cmd(data).await;
            stream.write(res.as_bytes()).await.ok();
          }
        }
      });
      return;
    }
    let stream = FramedStream::from(stream, addr);
    tokio::spawn(async move {
      let mut stream = stream;
      if let Some(Ok(bytes)) = stream.next_timeout(30_000).await {
        if let Ok(msg_in) = RendezvousMessage::parse_from_bytes(&bytes) {
          #[allow(clippy::single_match)]
          match msg_in.union {
            Some(rendezvous_message::Union::TestNatRequest(_)) => {
              let mut msg_out = RendezvousMessage::new();
              msg_out.set_test_nat_response(TestNatResponse {
                port: addr.port() as _,
                ..Default::default()
              });
              stream.send(&msg_out).await.ok();
            }
            _ => {}
          }
        }
      }
    });
  }
}
