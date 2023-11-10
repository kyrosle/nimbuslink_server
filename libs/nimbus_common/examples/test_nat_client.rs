use nimbus_common::ResultType;

fn main() -> ResultType<()> {
  nimbus_common::common::logger_initialize::logger_init!();
  nimbus_common::common::test_nat_type_client();
  Ok(())
}
