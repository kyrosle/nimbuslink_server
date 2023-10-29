use nimbus_common::ResultType;

use crate::rendezvous_server::RendezvousServer;

pub mod rendezvous_server;

fn main() -> ResultType<()> {
  nimbus_common::common::logger_initialize::init!();
  let port = 8080;
  RendezvousServer::start(port, 0)?;
  Ok(())
}
