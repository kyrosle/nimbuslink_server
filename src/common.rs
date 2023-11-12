use std::time::Instant;

pub(crate) fn get_expired_time() -> Instant {
  let now = Instant::now();
  now
    .checked_sub(std::time::Duration::from_secs(3600))
    .unwrap_or(now)
}
