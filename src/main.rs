use flexi_logger::{Logger, opt_format};
use nimbus_common::{log::info, ResultType};

fn main() -> ResultType<()> {
  let _logger = Logger::try_with_env_or_str("info")?
    .log_to_stdout()
    .format(opt_format)
    .write_mode(flexi_logger::WriteMode::Async)
    .start()?;
  info!("logger init");
  Ok(())
}
