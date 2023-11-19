use std::net::SocketAddr;

use nimbus_common::{
  allow_err,
  bytes::Bytes,
  futures::SinkExt,
  protobuf::Message,
  protos::rendezvous::{
    register_pk_response, rendezvous_message, RegisterPkResponse,
    RendezvousMessage, TestNatRequest, TestNatResponse,
  },
};

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
          self.handle_test_nat_request(&tar, addr, sink).await
        }
        Some(rendezvous_message::Union::RegisterPk(_)) => {
          self.handle_tcp_register_pk(sink).await
        }
        _ => {}
      }
    }
    false
  }

  pub(super) async fn handle_test_nat_request(
    &mut self,
    tar: &TestNatRequest,
    addr: SocketAddr,
    sink: &mut Option<Sink>,
  ) {
    let mut msg_out = RendezvousMessage::new();
    let res = TestNatResponse {
      port: addr.port() as _,
      ..Default::default()
    };
    if self.inner.serial > tar.serial {
      // config update
    }
    msg_out.set_test_nat_response(res);
    Self::send_to_sink(sink, msg_out).await;
  }

  pub(super) async fn handle_tcp_register_pk(
    &mut self,
    sink: &mut Option<Sink>,
  ) {
    let res = register_pk_response::Result::NOT_SUPPORT;
    let mut msg_out = RendezvousMessage::new();
    msg_out.set_register_pk_response(RegisterPkResponse {
      result: res.into(),
      ..Default::default()
    });
    Self::send_to_sink(sink, msg_out).await;
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
