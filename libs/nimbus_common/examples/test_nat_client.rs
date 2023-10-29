use nimbus_common::ResultType;

fn main() -> ResultType<()> {
  nimbus_common::common::logger_initialize::init!();
  nimbus_common::common::test_nat_type_client();
  Ok(())
}
