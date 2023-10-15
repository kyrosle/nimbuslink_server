use std::{
  net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
  time::{Duration, Instant},
};

use nimbus_common::{
  anyhow::bail,
  config,
  logger::*,
  protobuf::Message,
  protos::rendezvous::{RegisterPeer, RendezvousMessage},
  tokio::{self, time::interval},
  udp::FramedSocket,
  ResultType,
};

pub(crate) async fn test_nimbus(addr: SocketAddr) -> ResultType<()> {
  info!("test_nimbus");
  let mut addr = addr;
  if addr.ip().is_unspecified() {
    addr.set_ip(if addr.is_ipv4() {
      IpAddr::V4(Ipv4Addr::LOCALHOST)
    } else {
      IpAddr::V6(Ipv6Addr::LOCALHOST)
    })
  }

  let mut socket =
    FramedSocket::new(config::Config::get_any_listen_addr(addr.is_ipv4()))
      .await?;
  let mut msg_out = RendezvousMessage::new();
  msg_out.set_register_peer(RegisterPeer {
    id: "(:test_nimbus:)".to_owned(),
    ..Default::default()
  });
  let mut last_time_recv = Instant::now();

  let mut timer = interval(Duration::from_secs(1));

  loop {
    tokio::select! {
      _ = timer.tick() => {
        if last_time_recv.elapsed().as_secs() > 12 {
          bail!("Timeout of test_nimbus");
        }
        socket.send(&msg_out, addr).await?;
      }
      Some(Ok((bytes, _))) = socket.next() => {
        if let Ok(msg_in) = RendezvousMessage::parse_from_bytes(&bytes) {
          trace!("Recv {:?} of test_nimbus", msg_in);
          last_time_recv = Instant::now();
        }
      }
    }
  }
}
