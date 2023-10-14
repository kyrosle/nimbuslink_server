use std::net::SocketAddr;

use anyhow::Context;
use tokio_socks::TargetAddr;

use crate::{is_ipv4_str, is_ipv6_str, tcp::FramedStream, ResultType};

pub mod is_resolved_socket_addr;

#[inline]
pub fn check_port<T: std::string::ToString>(host: T, port: i32) -> String {
  let host = host.to_string();
  if crate::is_ipv6_str(&host) {
    if host.starts_with('[') {
      return host;
    }
    return format!("[{host}]:{port}");
  }
  if !host.contains(':') {
    return format!("{host}:{port}");
  }
  host
}

#[inline]
pub fn increase_port<T: std::string::ToString>(host: T, offset: i32) -> String {
  let host = host.to_string();
  if crate::is_ipv6_str(&host) {
    if host.starts_with('[') {
      let tmp = host.split("]:").collect::<Vec<&str>>();
      if tmp.len() == 2 {
        let port = tmp[1].parse().unwrap_or(0);
        if port > 0 {
          return format!("{}]:{}", tmp[0], port + offset);
        }
      }
    }
  } else if host.contains(':') {
    let tmp = host.split(':').collect::<Vec<&str>>();
    if tmp.len() == 2 {
      let port = tmp[1].parse().unwrap_or(0);
      if port > 0 {
        return format!("{}:{}", tmp[0], port + offset);
      }
    }
  }
  host
}

#[inline]
pub fn is_ipv4(target: &TargetAddr<'_>) -> bool {
  match target {
    TargetAddr::Ip(addr) => addr.is_ipv4(),
    _ => true,
  }
}

#[inline]
pub fn is_ip_str(id: &str) -> bool {
  is_ipv4_str(id) || is_ipv6_str(id)
}

#[inline]
pub async fn query_nip_io(addr: &SocketAddr) -> ResultType<SocketAddr> {
  tokio::net::lookup_host(format!("{}.nip.io:{}", addr.ip(), addr.port()))
    .await?
    .find(|x| x.is_ipv6())
    .context("Failed to get ipv6 from nip.io")
}

#[inline]
pub fn ipv4_to_ipv6(addr: String, ipv4: bool) -> String {
  if !ipv4 && crate::is_ipv4_str(&addr) {
    if let Some(ip) = addr.split(':').next() {
      return addr.replace(ip, &format!("{ip}.nip.io"));
    }
  }
  addr
}

async fn test_target(target: &str) -> ResultType<SocketAddr> {
  if let Ok(Ok(s)) =
    crate::timeout(1000, tokio::net::TcpStream::connect(target)).await
  {
    if let Ok(addr) = s.peer_addr() {
      return Ok(addr);
    }
  }
  tokio::net::lookup_host(target)
    .await?
    .next()
    .context(format!("Failed to lookup host for {target}"))
}
#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use std::net::ToSocketAddrs;

  #[test]
  fn test_nat64() {
    test_nat64_async();
  }

  #[tokio::main(flavor = "current_thread")]
  async fn test_nat64_async() {
    assert_eq!(ipv4_to_ipv6("1.1.1.1".to_owned(), true), "1.1.1.1");
    assert_eq!(ipv4_to_ipv6("1.1.1.1".to_owned(), false), "1.1.1.1.nip.io");
    assert_eq!(
      ipv4_to_ipv6("1.1.1.1:8080".to_owned(), false),
      "1.1.1.1.nip.io:8080"
    );
    assert_eq!(
      ipv4_to_ipv6("rustdesk.com".to_owned(), false),
      "rustdesk.com"
    );

    if ("rustdesk.com:80")
      .to_socket_addrs()
      .unwrap()
      .next()
      .unwrap()
      .is_ipv6()
    {
      assert!(query_nip_io(&"1.1.1.1:80".parse().unwrap())
        .await
        .unwrap()
        .is_ipv6());
    }

    assert!(query_nip_io(&"1.1.1.1:80".parse().unwrap()).await.is_err());
  }

  #[test]
  fn test_check_port() {
    assert_eq!(check_port("[1:2]:12", 32), "[1:2]:12");
    assert_eq!(check_port("1:2", 32), "[1:2]:32");
    assert_eq!(check_port("z1:2", 32), "z1:2");
    assert_eq!(check_port("1.1.1.1", 32), "1.1.1.1:32");
    assert_eq!(check_port("1.1.1.1:32", 32), "1.1.1.1:32");
    assert_eq!(check_port("test.com:32", 0), "test.com:32");
    assert_eq!(increase_port("[1:2]:12", 1), "[1:2]:13");
    assert_eq!(increase_port("1.2.2.4:12", 1), "1.2.2.4:13");
    assert_eq!(increase_port("1.2.2.4", 1), "1.2.2.4");
    assert_eq!(increase_port("test.com", 1), "test.com");
    assert_eq!(increase_port("test.com:13", 4), "test.com:17");
    assert_eq!(increase_port("1:13", 4), "1:13");
    assert_eq!(increase_port("22:1:13", 4), "22:1:13");
    assert_eq!(increase_port("z1:2", 1), "z1:3");
  }
}
