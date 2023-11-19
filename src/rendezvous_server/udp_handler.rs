use std::{collections::HashMap, net::SocketAddr, time::Instant};

use nimbus_common::{
  bytes::BytesMut,
  futures::StreamExt,
  logger::*,
  protobuf::Message,
  protos::rendezvous::{
    register_pk_response, rendezvous_message, ConfigUpdate, RegisterPeer,
    RegisterPeerResponse, RegisterPk, RegisterPkResponse, RendezvousMessage,
  },
  tokio_util::udp,
  udp::FramedSocket,
  ResultType,
};
use tungstenite::protocol::frame::Frame;

use crate::peer::{
  DAY_SECONDS, IP_BLOCKER, IP_BLOCK_DUR, IP_CHANGES, IP_CHANGE_DUR,
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
      match msg_in.union {
        Some(rendezvous_message::Union::RegisterPeer(rp)) => {
          self.handle_register_peer(rp, addr, udp_socket).await?
        }
        Some(rendezvous_message::Union::RegisterPk(rk)) => {
          self.handle_udp_register_pk(rk, addr, udp_socket).await?;
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
    let mut msg_out = RendezvousMessage::new();
    if self.inner.serial > rp.serial {
      msg_out.set_configure_update(ConfigUpdate {
        serial: self.inner.serial,
        rendezvous_servers: (*self.rendezvous_servers).clone(),
        ..Default::default()
      });
      udp_socket.send(&msg_out, addr).await?;
    }
    Ok(())
  }

  /// message RegisterPk {
  ///   string id = 1;
  ///   bytes uuid = 2;
  ///   bytes pk = 3; // public key
  ///   string old_id = 4;
  /// }
  pub(super) async fn handle_udp_register_pk(
    &mut self,
    rk: RegisterPk,
    addr: SocketAddr,
    udp_socket: &mut FramedSocket,
  ) -> ResultType<()> {
    if rk.uuid.is_empty() || rk.pk.is_empty() {
      return Ok(());
    }
    let id = rk.id;
    let ip = addr.ip().to_string();
    // id should be at least 6 chars
    if id.len() < 6 {
      // uuid mismatch
      return send_rk_res(
        udp_socket,
        addr,
        register_pk_response::Result::UUID_MISMATCH,
      )
      .await;
    } else if !self.check_ip_blocker(&ip, &id).await {
      // too frequent
      return send_rk_res(
        udp_socket,
        addr,
        register_pk_response::Result::TOO_FREQUENT,
      )
      .await;
    }

    // retrieves a peer based on id
    // if the peer is new, return a new LockPeer
    let peer = self.peer_map.get_or(&id).await;

    let (changed, ip_changed) = {
      let peer = peer.read().await;
      if peer.uuid.is_empty() {
        (true, false)
      } else {
        // whether the peer uuid, ip and public_key is same as the register_pk message
        if peer.uuid == rk.uuid {
          if peer.peer_info.ip != ip && peer.pk != rk.pk {
            warn!(
              "Peer {} ip/pk mismatch: {}/{:?} vs {}/{:?}",
              id, ip, rk.pk, peer.peer_info.ip, peer.pk
            );
            drop(peer);
            return send_rk_res(
              udp_socket,
              addr,
              register_pk_response::Result::UUID_MISMATCH,
            )
            .await;
          }
        } else {
          warn!(
            "Peer {} uuid mismatch: {:?} vs {:?}",
            id, rk.uuid, peer.uuid
          );
          drop(peer);
          return send_rk_res(
            udp_socket,
            addr,
            register_pk_response::Result::UUID_MISMATCH,
          )
          .await;
        }

        // whether the peer address ip has changed
        let ip_changed = peer.peer_info.ip != ip;
        (
          peer.uuid != rk.uuid || peer.pk != rk.pk || ip_changed,
          ip_changed,
        )
      }
    };

    // (counter counts, Instant) avoid multiple register_pk requests in a short time
    let mut req_pk = peer.read().await.reg_pk;
    if req_pk.1.elapsed().as_secs() > 6 {
      req_pk.0 = 0;
    } else if req_pk.0 > 2 {
      return send_rk_res(
        udp_socket,
        addr,
        register_pk_response::Result::TOO_FREQUENT,
      )
      .await;
    }

    req_pk.0 += 1;
    req_pk.1 = Instant::now();

    peer.write().await.reg_pk = req_pk;

    if ip_changed {
      let mut lock = IP_CHANGES.lock().await;
      if let Some((tm, ips)) = lock.get_mut(&id) {
        // tracking the frequency of Ip address changes
        if tm.elapsed().as_secs() > IP_CHANGE_DUR {
          // the specified duration for Ip address changed has passed.
          *tm = Instant::now();
          ips.clear();
          ips.insert(ip.clone(), 1);
        } else if let Some(v) = ips.get_mut(&ip) {
          // the specified duration for Ip address changed has not passed.
          *v += 1;
        } else {
          // new ip
          ips.insert(ip.clone(), 1);
        }
      } else {
        lock.insert(
          id.clone(),
          (Instant::now(), HashMap::from([(ip.clone(), 1)])),
        );
      }
    }

    // update the peer information
    if changed {
      self
        .peer_map
        .update_pk(id, peer, addr, rk.uuid, rk.pk, ip)
        .await;
    }

    let mut msg_out = RendezvousMessage::new();

    // response to the registering peer
    msg_out.set_register_pk_response(RegisterPkResponse {
      result: register_pk_response::Result::OK.into(),
      ..Default::default()
    });

    udp_socket.send(&msg_out, addr).await
  }

  /// check if an IP address is blocked based on certain conditions
  async fn check_ip_blocker(&self, ip: &str, id: &str) -> bool {
    // 1. required the IP_BLOCKER lock,
    let mut lock = IP_BLOCKER.lock().await;
    let now = Instant::now();
    // check if the ip address exists in the `IP_BLOCKER` map
    if let Some(old) = lock.get_mut(ip) {
      // - Ip blocking counter (u32, Instant)
      let counter = &mut old.0;
      // 2. check if the elapsed time since the last IP block counter reset
      //    is greater than the `IP_BLOCK_DUR` duration.
      // 3. check the blocking counter status:
      //     - longer then `BIP_BLOCK_DUR`, reset the blocking counter as 0
      //     - check the blocking counter counts, if greater than 30, it will consider as blocked
      if counter.1.elapsed().as_secs() > IP_BLOCK_DUR {
        counter.0 = 0;
      } else if counter.0 > 30 {
        return false;
      }

      // 4. if the ip address was not blocked, increase the blocking counter,
      counter.0 += 1;
      counter.1 = now;

      // 5. get the relevant id counter set (HashSet<id>, Instant)
      let counter = &mut old.1;
      let is_new = counter.0.get(id).is_none();
      // elapsed one day (3600 * 24)
      if counter.1.elapsed().as_secs() > DAY_SECONDS {
        counter.0.clear();
      } else if counter.0.len() > 300 {
        // larger then limited(300), if the Ip not the new one, it will be considered as blocked
        return !is_new;
      }

      if is_new {
        // added to the id counter set
        counter.0.insert(id.to_owned());
      }
      // update the last visited time
      counter.1 = now;
    } else {
      lock.insert(ip.to_owned(), ((0, now), (Default::default(), now)));
    }
    true
  }
}

#[inline]
async fn send_rk_res(
  socket: &mut FramedSocket,
  addr: SocketAddr,
  res: register_pk_response::Result,
) -> ResultType<()> {
  let mut msg_out = RendezvousMessage::new();
  msg_out.set_register_pk_response(RegisterPkResponse {
    result: res.into(),
    ..Default::default()
  });
  socket.send(&msg_out, addr).await
}
