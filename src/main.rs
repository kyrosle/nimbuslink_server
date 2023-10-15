use nimbus_common::{log::info, ResultType};
use flexi_logger::{colored_opt_format, Logger};
use nimbus_common::{logger::*, ResultType};

use crate::rendezvous_server::RendezvousServer;

pub mod rendezvous_server;
fn main() -> ResultType<()> {
  let _logger = Logger::try_with_env_or_str("info")?
    .log_to_stdout()
    .set_palette("196;208;80;7;8".to_string())
    .format(colored_opt_format)
    .write_mode(flexi_logger::WriteMode::Async)
    .start()?;
  info!("logger init finished");

  let port = 8080;
  RendezvousServer::start(port, 0)?;
  Ok(())
}
