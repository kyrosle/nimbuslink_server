#[macro_export]
macro_rules! allow_err {
  ($e: expr) => {
    if let Err(err) = $e {
      nimbus_common::logger::debug!(
        "{:?}, {}:{}:{}:{}",
        err,
        module_path!(),
        file!(),
        line!(),
        column!()
      );
    } else {
    }
  };
}
