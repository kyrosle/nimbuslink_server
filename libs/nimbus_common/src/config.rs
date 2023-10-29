use once_cell::sync::Lazy;
use std::{
  net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
  sync::{Arc, RwLock},
};

use crate::protos::rendezvous::NatType;

// compressor name     |ratio|compression|decompression
// zstd 1.5.1 -1	     |2.887|530 MB/s	 |1700 MB/s
// zstd 1.5.1 --fast=1 |2.437|600 MB/s	 |2150 MB/s
// zstd 1.5.1 --fast=3 |2.239|670 MB/s	 |2250 MB/s <-
// zstd 1.5.1 --fast=4 |2.148|710 MB/s	 |2300 MB/s
pub const COMPRESS_LEVEL: i32 = 3;
pub const CONNECT_TIMEOUT: u64 = 18_000;
pub const READ_TIMEOUT: u64 = 18_000;
pub const SERIAL: i32 = 3;

// global static variable
static CONFIG: Lazy<Arc<RwLock<Config>>> =
  Lazy::new(|| Arc::new(RwLock::new(Config {})));
static CONFIG2: Lazy<Arc<RwLock<Config2>>> = Lazy::new(|| {
  Arc::new(RwLock::new(Config2 {
    socks: None,
    serial: SERIAL,
    nat_type: NatType::UNKNOWN_NAT as _,
  }))
});

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NetWorkType {
  Direct,
  ProxySocks,
}

pub struct Config {}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Socks5Server {
  pub proxy: String,
  pub username: String,
  pub password: String,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Config2 {
  serial: i32,
  socks: Option<Socks5Server>,
  nat_type: i32,
}

impl Config {
  pub fn get_socks() -> Option<Socks5Server> {
    CONFIG2.read().unwrap().socks.clone()
  }

  pub fn get_serial() -> i32 {
    std::cmp::max(CONFIG2.read().unwrap().serial, SERIAL)
  }

  pub fn get_nat_type() -> i32 {
    CONFIG2.read().unwrap().nat_type
  }

  pub fn set_nat_type(nat_type: i32) {
    let mut config = CONFIG2.write().unwrap();
    if nat_type == config.nat_type {
      return;
    }
    config.nat_type = nat_type;
    // config.store();
  }

  #[inline]
  pub fn get_any_listen_addr(is_ipv4: bool) -> SocketAddr {
    if is_ipv4 {
      SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)
    } else {
      SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0)
    }
  }

  pub fn get_network_type() -> NetWorkType {
    match &CONFIG2.read().unwrap().socks {
      None => NetWorkType::Direct,
      Some(_) => NetWorkType::ProxySocks,
    }
  }
}
