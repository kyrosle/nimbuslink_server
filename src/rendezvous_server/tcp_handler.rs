use std::net::SocketAddr;

use nimbus_common::{
  bytes::Bytes,
  futures::SinkExt,
  protobuf::Message,
  protos::rendezvous::{
    rendezvous_message, RendezvousMessage, TestNatResponse,
  },
};
use nimbuslink_server::allow_err;

use super::{RendezvousServer, Sink};

impl RendezvousServer {
  #[inline]
  pub(super) async fn handle_tcp(
    &mut self,
    bytes: &[u8],
    sink: &mut Option<Sink>,
    addr: SocketAddr,
    key: &str,
    is_websocket: bool,
  ) -> bool {
    if let Ok(msg_in) = RendezvousMessage::parse_from_bytes(bytes) {
      match msg_in.union {
        Some(rendezvous_message::Union::TestNatRequest(tar)) => {
          let mut msg_out = RendezvousMessage::new();
          let mut res = TestNatResponse {
            port: addr.port() as _,
            ..Default::default()
          };
          if self.inner.serial > tar.serial {
            // config update
          }
          msg_out.set_test_nat_response(res);
          Self::send_to_sink(sink, msg_out).await;
        }
        _ => {}
      }
    }
    false
  }

  #[inline]
  async fn send_to_sink(sink: &mut Option<Sink>, msg: RendezvousMessage) {
    if let Some(sink) = sink.as_mut() {
      if let Ok(bytes) = msg.write_to_bytes() {
        match sink {
          Sink::TcpStream(s) => {
            allow_err!(s.send(Bytes::from(bytes)).await)
          }
          Sink::Ws(ws) => {
            allow_err!(ws.send(tungstenite::Message::Binary(bytes)).await)
          }
        }
      }
    }
  }
}
