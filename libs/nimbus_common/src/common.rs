use std::net::SocketAddr;

use flexi_logger::{
  self, colored_opt_format, FlexiLoggerError, Logger, LoggerHandle,
};
use protobuf::Message;

use crate::config::{Config, CONNECT_TIMEOUT, READ_TIMEOUT};
use crate::tcp::FramedStream;
use crate::{logger::*, socket_client};

use crate::protos::rendezvous::{
  rendezvous_message, NatType, RendezvousMessage, TestNatRequest,
};
use crate::ResultType;

pub mod logger_initialize {
  #[macro_export]
  macro_rules! init {
    () => {
      let _logger =
        nimbus_common::flexi_logger::Logger::try_with_env_or_str("debug")?
          .log_to_stdout()
          .set_palette("196;208;7;80;8".to_string())
          .format(nimbus_common::flexi_logger::colored_opt_format)
          .write_mode(nimbus_common::flexi_logger::WriteMode::Async)
          .start()?;
      nimbus_common::logger::info!("logger init finished");
    };
  }
  pub use init;
}

// pub fn logger_initialize() -> ResultType<()> {
//   Ok(())
// }

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TestNatPort {
  Port1,
  Port2,
}

pub fn test_nat_type_client() {
  let mut retest_interval = 0;
  std::thread::spawn(move || loop {
    match test_nat_type_client_() {
      Ok(true) => break,
      Err(err) => {
        error!("Test nat: {}", err);
      }
      _ => {}
    }
    if Config::get_nat_type() != 0 {
      break;
    }
    retest_interval = retest_interval * 2 + 1;
    if retest_interval > 300 {
      retest_interval = 300;
    }
    std::thread::sleep(std::time::Duration::from_secs(retest_interval));
  })
  .join()
  .unwrap_or_else(|e| {
    error!("Test nat error: {:?}", e);
  });
}

#[tokio::main(flavor = "current_thread")]
async fn test_nat_type_client_() -> ResultType<bool> {
  info!("Testing nat...");
  // android or ios:
  let is_direct = Config::get_socks().is_none();
  // otherwise

  if !is_direct {
    Config::set_nat_type(NatType::SYMMETRIC as _);
    return Ok(true);
  }

  let start = std::time::Instant::now();
  // let (rendezvous_server, _, _) =
  let server1 = "127.0.0.1:8080";
  let server2 = increase_port(server1, -1);

  let mut msg_out = RendezvousMessage::new();
  let serial = Config::get_serial();
  msg_out.set_test_nat_request(TestNatRequest {
    serial,
    ..Default::default()
  });

  let mut local_addr = None;
  let port1 =
    test_port(server1, &msg_out, &mut local_addr, TestNatPort::Port1).await?;
  let port2 =
    test_port(server2, &msg_out, &mut local_addr, TestNatPort::Port2).await?;

  let ok = port1.is_some_and(|p| p > 0) && port2.is_some_and(|p| p > 0);
  if ok {
    let t = if port1 == port2 {
      NatType::ASYMMETRIC
    } else {
      NatType::SYMMETRIC
    };
    // config set 'nat-type'
    info!("Tested nat type: {:?} in {:?}", t, start.elapsed());
  }

  Ok(ok)
}

// pub async fn get_rendezvous_server(ms_timeout: u64) -> (String, Vec<String>, bool) {
//   // android or ios:
//   let (mut a, mut b, ) = get_rendezvous_server_(ms_timeout);
// }

#[inline]
async fn test_port<S: Into<String> + std::fmt::Display + Clone>(
  server: S,
  msg_out: &RendezvousMessage,
  local_addr: &mut Option<SocketAddr>,
  test_nat_port_type: TestNatPort,
) -> ResultType<Option<i32>> {
  let mut port = None;
  let mut socket = socket_client::connect_tcp_local(
    server.clone().into(),
    *local_addr,
    CONNECT_TIMEOUT,
  )
  .await?;

  if test_nat_port_type == TestNatPort::Port1 {
    // reuse the local addr is required for nat test
    *local_addr = Some(socket.local_addr());
    // config set option 'local-ip-addr'
  }

  socket.send(msg_out).await?;

  if let Some(msg_in) = get_next_non_key_exchange_msg(&mut socket, None).await {
    if let Some(rendezvous_message::Union::TestNatResponse(tnr)) = msg_in.union
    {
      debug!("Got nat response from {}: port={}", server, tnr.port);
      port = Some(tnr.port);
      // config update 'rendezvous-servers', 'serial'
    }
  }
  Ok(port)
}

#[inline]
pub async fn get_next_non_key_exchange_msg(
  conn: &mut FramedStream,
  timeout: Option<u64>,
) -> Option<RendezvousMessage> {
  let timeout = timeout.unwrap_or(READ_TIMEOUT);
  if let Some(Ok(bytes)) = conn.next_timeout(timeout).await {
    if let Ok(msg_in) = RendezvousMessage::parse_from_bytes(&bytes) {
      match &msg_in.union {
        // ignore KeyExchange protobuf message
        _ => return Some(msg_in),
      }
    }
  }
  None
}

#[inline]
pub fn increase_port<T: std::string::ToString>(host: T, offset: i32) -> String {
  socket_client::increase_port(host, offset)
}
