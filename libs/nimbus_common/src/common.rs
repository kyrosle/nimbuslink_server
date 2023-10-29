use flexi_logger::{
  self, colored_opt_format, FlexiLoggerError, Logger, LoggerHandle,
};
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
