use std::cell::RefCell;

use zstd::bulk::{Compressor, Decompressor};

// zstd manual page: https://facebook.github.io/zstd/zstd_manual.html
thread_local! {
  static COMPRESSOR : RefCell<std::io::Result<Compressor<'static>>> =
    RefCell::new(Compressor::new(crate::config::COMPRESS_LEVEL));
  static DECOMPRESSOR: RefCell<std::io::Result<Decompressor<'static>>> =
    RefCell::new(Decompressor::new());
}

pub fn compress(data: &[u8]) -> Vec<u8> {
  let mut out = Vec::new();
  COMPRESSOR.with(|c| {
    if let Ok(mut c) = c.try_borrow_mut() {
      match &mut *c {
        Ok(c) => match c.compress(data) {
          Ok(res) => out = res,
          Err(err) => {
            crate::log::debug!("Failed to compress: {}", err);
          }
        },
        Err(err) => {
          crate::log::debug!("Failed to get compressor: {}", err);
        }
      }
    }
  });
  out
}

pub fn decompress(data: &[u8]) -> Vec<u8> {
  let mut out = Vec::new();
  DECOMPRESSOR.with(|d| {
    if let Ok(mut d) = d.try_borrow_mut() {
      match &mut *d {
        Ok(d) => {
          const MAX: usize = 1024 * 1024 * 64;
          const MIN: usize = 1024 * 1024;
          let mut n = 30 * data.len();
          n = n.clamp(MIN, MAX);
          match d.decompress(data, n) {
            Ok(res) => out = res,
            Err(err) => {
              crate::log::debug!("Failed to decompress: {}", err);
            }
          }
        }
        Err(err) => {
          crate::log::debug!("Failed to get decompressor: {}", err);
        }
      }
    }
  });
  out
}
