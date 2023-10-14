use once_cell::sync::Lazy;
use std::{
  net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
  sync::{Arc, RwLock},
};

// compressor name     |ratio|compression|decompression
// zstd 1.5.1 -1	     |2.887|530 MB/s	 |1700 MB/s
// zstd 1.5.1 --fast=1 |2.437|600 MB/s	 |2150 MB/s
// zstd 1.5.1 --fast=3 |2.239|670 MB/s	 |2250 MB/s <-
// zstd 1.5.1 --fast=4 |2.148|710 MB/s	 |2300 MB/s
pub const COMPRESS_LEVEL: i32 = 3;

// global static variable
static CONFIG: Lazy<Arc<RwLock<Config>>> =
  Lazy::new(|| Arc::new(RwLock::new(Config {})));
static CONFIG2: Lazy<Arc<RwLock<Config2>>> =
  Lazy::new(|| Arc::new(RwLock::new(Config2 { socks: None })));

pub enum NetWorkType {
  Direct,
  ProxySocks,
}

pub struct Config {}

pub struct Socks5Server {
  pub proxy: String,
  pub username: String,
  pub password: String,
}

pub struct Config2 {
  pub socks: Option<Socks5Server>,
}

impl Config {
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
