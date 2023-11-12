use nimbus_common::ResultType;

use nimbuslink_server::rendezvous_server::RendezvousServer;

fn main() -> ResultType<()> {
  nimbus_common::common::logger_initialize::logger_init!();
  let port = 8080;
  RendezvousServer::start(port, 0)?;
  Ok(())
}
