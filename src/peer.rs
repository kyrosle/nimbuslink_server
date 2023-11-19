use std::{
  collections::{HashMap, HashSet},
  net::SocketAddr,
  sync::Arc,
  time::Instant,
};

use nimbus_common::{
  bytes::Bytes,
  logger::*,
  once_cell::sync::Lazy,
  protos::rendezvous::register_pk_response,
  tokio::sync::{Mutex, RwLock},
  ResultType,
};
use serde_derive::{Deserialize, Serialize};

use crate::common::get_expired_time;

/// IP blocking information
///
/// For:
/// 1. Security
/// 2. Content Filtering
/// 3. DDos Mitigation
/// 4. User Management
/// [`check_ip_blocker`]
type IpBlockMap = HashMap<String, ((u32, Instant), (HashSet<String>, Instant))>;
/// User status information
type UserStatusMap = HashMap<Vec<u8>, Arc<(Option<Vec<u8>>, bool)>>;
/// Ip change information
type IpChangesMap = HashMap<String, (Instant, HashMap<String, i32>)>;
pub(crate) static IP_BLOCKER: Lazy<Mutex<IpBlockMap>> =
  Lazy::new(Default::default);
pub(crate) static USER_STATUS: Lazy<RwLock<UserStatusMap>> =
  Lazy::new(Default::default);
pub(crate) static IP_CHANGES: Lazy<Mutex<IpChangesMap>> =
  Lazy::new(Default::default);
pub static IP_CHANGE_DUR: u64 = 180;
pub static IP_CHANGE_DUR_X2: u64 = IP_CHANGE_DUR * 2;
pub static DAY_SECONDS: u64 = 3600 * 24;
pub static IP_BLOCK_DUR: u64 = 60;

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub(crate) struct PeerInfo {
  #[serde(default)]
  pub(crate) ip: String,
}

pub(crate) struct Peer {
  pub(crate) socket_addr: SocketAddr,
  pub(crate) last_register_time: Instant,
  pub(crate) guid: Vec<u8>,
  pub(crate) uuid: Bytes,
  pub(crate) pk: Bytes,
  pub(crate) peer_info: PeerInfo,
  pub(crate) reg_pk: (u32, Instant),
}

pub(crate) type LockPeer = Arc<RwLock<Peer>>;

impl Default for Peer {
  fn default() -> Self {
    Peer {
      socket_addr: "0.0.0.0".parse().unwrap(),
      last_register_time: get_expired_time(),
      guid: Vec::new(),
      uuid: Bytes::new(),
      pk: Bytes::new(),
      peer_info: Default::default(),
      reg_pk: (0, get_expired_time()),
    }
  }
}

#[derive(Clone)]
pub(crate) struct PeerMap {
  map: Arc<RwLock<HashMap<String, LockPeer>>>,
  // TODO: sqlx support
}

impl PeerMap {
  pub(crate) async fn new() -> ResultType<Self> {
    let pm = PeerMap {
      map: Default::default(),
    };
    Ok(pm)
  }

  #[inline]
  pub(crate) async fn update_pk(
    &mut self,
    id: String,
    peer: LockPeer,
    addr: SocketAddr,
    uuid: Bytes,
    pk: Bytes,
    ip: String,
  ) -> register_pk_response::Result {
    info!("update_pk {} {:?} {:?} {:?}", id, addr, uuid, pk);

    let (info_str, guid) = {
      let mut w = peer.write().await;
      w.socket_addr = addr;
      w.uuid = uuid.clone();
      w.pk = pk.clone();
      w.last_register_time = get_expired_time();
      w.peer_info.ip = ip;

      (
        serde_json::to_string(&w.peer_info).unwrap_or_default(),
        w.guid.clone(),
      )
    };

    if guid.is_empty() {
      // TODO: insert record to sqlx
      peer.write().await.guid = guid;
    } else {
      // TODO: update the record in sqlx
      info!("pk updated instead of insert");
    }

    register_pk_response::Result::OK
  }

  #[inline]
  pub(crate) async fn get(&self, id: &str) -> Option<LockPeer> {
    let p = self.map.read().await.get(id).cloned();
    if p.is_some() {
      return p;
    } else {
      // TODO: get peer from sqlx
    }
    None
  }

  #[inline]
  pub(crate) async fn get_or(&self, id: &str) -> LockPeer {
    if let Some(p) = self.get(id).await {
      return p;
    }
    let mut w = self.map.write().await;
    if let Some(p) = w.get(id) {
      return p.clone();
    }
    let tmp = LockPeer::default();
    w.insert(id.to_owned(), tmp.clone());
    tmp
  }

  #[inline]
  pub(crate) async fn get_in_memory(&self, id: &str) -> Option<LockPeer> {
    self.map.read().await.get(id).cloned()
  }

  #[inline]
  pub(crate) async fn is_in_memory(&self, id: &str) -> bool {
    self.map.read().await.contains_key(id)
  }
}
