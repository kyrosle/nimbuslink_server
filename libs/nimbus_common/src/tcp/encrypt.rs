use std::io::ErrorKind;

use bytes::{BufMut, BytesMut};
use sodiumoxide::crypto::{
  box_,
  secretbox::{self, Key},
};

use super::FramedStream;
use crate::ResultType;

#[derive(Clone)]
/// Symmetric authenticated encryption.
///
/// using the cryptographic tools library: https://github.com/jedisct1/libsodium
pub struct Encrypt(Key, u64, u64);

impl Encrypt {
  pub fn new(key: Key) -> Self {
    Encrypt(key, 0, 0)
  }

  pub fn decrypt(
    &mut self,
    bytes: &mut BytesMut,
  ) -> Result<(), std::io::Error> {
    self.2 += 1;
    let nonce = FramedStream::get_nonce(self.2);
    // verifies and decrypts a ciphertext `bytes` using a secret key and a nonce
    match secretbox::open(bytes, &nonce, &self.0) {
      Ok(res) => {
        // res: plaintext
        bytes.clear();
        bytes.put_slice(&res);
        Ok(())
      }
      Err(()) => Err(std::io::Error::new(ErrorKind::Other, "decryption error")),
    }
  }

  pub fn encrypt(&mut self, data: &[u8]) -> Vec<u8> {
    self.1 += 1;
    // encrypts and authenticates a message `data` using a secret key and a nonce
    let nonce = FramedStream::get_nonce(self.1);
    secretbox::seal(data, &nonce, &self.0)
  }

  pub fn decode(
    symmetric_data: &[u8], /* ciphertext */
    their_pk_b: &[u8],
    our_sk_b: &box_::SecretKey,
  ) -> ResultType<Key> {
    if their_pk_b.len() != box_::PUBLICKEYBYTES {
      anyhow::bail!("Handshake failed: pk length {}", their_pk_b.len());
    }
    let nonce = box_::Nonce([0u8; box_::NONCEBYTES]);
    let mut pk_ = [0u8; box_::PUBLICKEYBYTES];
    pk_[..].copy_from_slice(their_pk_b);
    let their_pk_b = box_::PublicKey(pk_);
    // verifies and decrypts
    let symmetric_key =
      box_::open(symmetric_data, &nonce, &their_pk_b, our_sk_b).map_err(
        |_| anyhow::anyhow!("Handshake failed: box decryption failure"),
      )?;
    if symmetric_key.len() != secretbox::KEYBYTES {
      anyhow::bail!("Handshake failed: invalid secret key length from peer");
    }

    let mut key = [0u8; secretbox::KEYBYTES];
    key[..].copy_from_slice(&symmetric_key);
    Ok(Key(key))
  }
}
