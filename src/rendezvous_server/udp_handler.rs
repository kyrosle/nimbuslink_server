use std::net::SocketAddr;

use nimbus_common::{
  bytes::BytesMut,
  logger::*,
  protobuf::Message,
  protos::rendezvous::{
    rendezvous_message, RegisterPeer, RegisterPeerResponse, RendezvousMessage,
  },
  udp::FramedSocket,
  ResultType,
};

use super::RendezvousServer;

impl RendezvousServer {
  #[inline]
  pub(super) async fn handle_udp(
    &mut self,
    bytes: &BytesMut,
    addr: SocketAddr,
    udp_socket: &mut FramedSocket,
  ) -> ResultType<()> {
    if let Ok(msg_in) = RendezvousMessage::parse_from_bytes(bytes) {
      #[allow(clippy::single_match)] /* FIXME: for now */
      match msg_in.union {
        Some(rendezvous_message::Union::RegisterPeer(rp)) => {
          self.handle_register_peer(rp, addr, udp_socket).await?
        }
        _ => {}
      }
    }
    Ok(())
  }

  pub(super) async fn handle_register_peer(
    &self,
    rp: RegisterPeer,
    addr: SocketAddr,
    udp_socket: &mut FramedSocket,
  ) -> ResultType<()> {
    debug!("Rendezvous Message RegisterPeer from {}: {:?}", addr, rp);
    let mut msg_out = RendezvousMessage::new();
    msg_out.set_register_peer_response(RegisterPeerResponse::default());
    udp_socket.send(&msg_out, addr).await?;
    Ok(())
  }
}
