use std::{
  net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
  sync::Arc,
  time::Duration,
};

mod test_nimbus;
mod udp_handler;
use udp_handler::*;
mod port_listener_handler;
use port_listener_handler::*;
mod nat_listener_handler;
use nat_listener_handler::*;
mod ws_listener_handler;
use ws_listener_handler::*;
mod tcp_handler;
use tcp_handler::*;

use nimbus_common::{
  bytes::Bytes,
  bytes_codec::BytesCodec,
  config::SERIAL,
  futures::stream::SplitSink,
  logger::*,
  tcp::listen_any,
  tokio::{
    self,
    net::{TcpListener, TcpStream},
    time::interval,
  },
  tokio_util::codec::Framed,
  udp::FramedSocket,
  ResultType,
};

type TcpStreamSink = SplitSink<Framed<TcpStream, BytesCodec>, Bytes>;
type WsSink = SplitSink<
  tokio_tungstenite::WebSocketStream<TcpStream>,
  tungstenite::Message,
>;
static CHECK_RELAY_TIMEOUT: u64 = 3_000;

enum Sink {
  TcpStream(TcpStreamSink),
  Ws(WsSink),
}

struct Inner {
  serial: i32,
  local_ip: String,
}

#[derive(Clone)]
pub struct RendezvousServer {
  inner: Arc<Inner>,
}

enum LoopFailure {
  UdpSocket,
  WsListener,
  NatListener,
  PortListener,
}

impl RendezvousServer {
  #[tokio::main(flavor = "multi_thread")]
  pub async fn start(port: i32, udp_recv_buffer_size: usize) -> ResultType<()> {
    let nat_port = port - 1;
    let ws_port = port + 2;
    debug!(
      "Rendezvous start with nat_port={}, ws_port={}",
      nat_port, ws_port
    );

    info!("Listening on tcp/udp: {}", port);
    info!("Listening on tcp: {}, extra port for NAT test", nat_port);
    info!("Listening on websocket: {}", ws_port);

    // udp socket
    let mut udp_socket = crate_udp_listener(port, udp_recv_buffer_size).await?;

    let local_ip = local_ip_address::local_ip()
      .map(|x| x.to_string())
      .unwrap_or_default();

    let mut rendezvous_server = RendezvousServer {
      inner: Arc::new(Inner {
        local_ip,
        serial: SERIAL,
      }),
    };

    let mut port_listener = create_tcp_listener(port).await?;
    let mut nat_listener = create_tcp_listener(nat_port).await?;
    let mut ws_listener = create_tcp_listener(ws_port).await?;

    let test_addr = port_listener.local_addr()?;
    // test
    tokio::spawn(async move {
      info!("first test address: {}", test_addr);
      if let Err(err) = test_nimbus::test_nimbus(test_addr).await {
        if test_addr.is_ipv6() && test_addr.ip().is_unspecified() {
          let mut test_addr = test_addr;
          test_addr.set_ip(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

          info!("second test address: {}", test_addr);
          if let Err(err) = test_nimbus::test_nimbus(test_addr).await {
            error!("Failed to run test_nimbus with {}: {}", test_addr, err);
            std::process::exit(1);
          }
        } else {
          error!("Failed to run test_nimbus with {}: {}", test_addr, err);
          std::process::exit(1);
        }
      }
    });

    let main_task = async move {
      loop {
        info!("main task start");
        match rendezvous_server
          .io_loop(
            &mut port_listener,
            &mut nat_listener,
            &mut ws_listener,
            &mut udp_socket,
          )
          .await
        {
          LoopFailure::UdpSocket => {
            debug!("LoopFailure UdpSocket");
            drop(udp_socket);
            udp_socket = crate_udp_listener(port, udp_recv_buffer_size).await?;
          }
          LoopFailure::WsListener => {
            debug!("LoopFailure WebSocket listener");
            drop(ws_listener);
            ws_listener = create_tcp_listener(port).await?;
          }
          LoopFailure::NatListener => {
            debug!("LoopFailure Nat listener");
            drop(nat_listener);
            nat_listener = create_tcp_listener(port).await?;
          }
          LoopFailure::PortListener => {
            debug!("LoopFailure Port tcp listener");
            drop(port_listener);
            port_listener = create_tcp_listener(port).await?;
          }
        }
      }
    };

    // TODO: add signal listener
    tokio::select! {
      res = main_task => res,
    }
  }

  async fn io_loop(
    &mut self,
    port_listener: &mut TcpListener,
    nat_listener: &mut TcpListener,
    ws_listener: &mut TcpListener,
    udp_socket: &mut FramedSocket,
  ) -> LoopFailure {
    let mut timer_check_relay =
      interval(Duration::from_millis(CHECK_RELAY_TIMEOUT));
    loop {
      tokio::select! {
        _ = timer_check_relay.tick() => {
          trace!("timer check relay");
        }
        res = port_listener.accept() => {
          match res {
            Ok((stream, addr)) => {
              stream.set_nodelay(true).ok();
              self.handle_port_listener(stream, addr, "").await;
            }
            Err(err) => {
              error!("port listener accept failure: {}", err);
              return LoopFailure::PortListener;
            }
          }
        }
        res = nat_listener.accept() => {
          match res {
            Ok((stream, addr)) => {
              stream.set_nodelay(true).ok();
              self.handle_nat_listener(stream, addr).await;
            }
            Err(err) => {
              error!("nat listener accept failure: {}", err);
              return LoopFailure::NatListener;
            }
          }
        }
        res = ws_listener.accept() => {
          match res {
            Ok((stream, addr)) => {
              stream.set_nodelay(true).ok();
              self.handle_ws_listener(stream, addr, "").await;
            }
            Err(err) => {
              error!("websocket listener accept failure: {}", err);
              return LoopFailure::WsListener;
            }
          }
        }
        res = udp_socket.next() => {
          match res {
            Some(Ok((bytes, addr))) => {
              if let Err(err) = self.handle_udp(&bytes, addr.into(), udp_socket).await {
                error!("udp failure: {}", err);
                return LoopFailure::UdpSocket;
              }
            }
            Some(Err(err)) => {
                error!("udp failure: {}", err);
                return LoopFailure::UdpSocket;
            }
            None => {
              unreachable!()
            }
          }
        }
      }
    }
  }

  async fn check_cmd(&self, cmd: &str) -> String {
    use std::fmt::Write as _;

    let mut res = "".to_owned();
    let mut fds = cmd.trim().split(' ');
    match fds.next() {
      Some("h") => {}
      Some("relay-servers" | "rs") => {}
      Some("ip-blocker" | "ib") => {}
      Some("ip-change" | "ic") => {}
      Some("always-use-relay" | "aur") => {}
      Some("test-geo" | "tg") => {}
      _ => {}
    }
    res
  }
}

async fn crate_udp_listener(
  port: i32,
  recv_buffer_size: usize,
) -> ResultType<FramedSocket> {
  let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port as _);
  info!("try to create udp FramedSocket on ipv6: {}", addr);
  if let Ok(s) = FramedSocket::new_reuse(&addr, false, recv_buffer_size).await {
    debug!("listen on udp {:?}", s.local_addr());
    return Ok(s);
  }

  let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port as _);
  info!("try to create udp FramedSocket on ipv4: {}", addr);
  let s = FramedSocket::new_reuse(&addr, false, recv_buffer_size).await?;
  debug!("listen on tcp {:?}", s.local_addr());
  Ok(s)
}

#[inline]
async fn create_tcp_listener(port: i32) -> ResultType<TcpListener> {
  let s = listen_any(port as _).await?;
  debug!("listen on tcp {:?}", s.local_addr());
  Ok(s)
}
