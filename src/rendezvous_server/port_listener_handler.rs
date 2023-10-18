use std::net::SocketAddr;

use nimbus_common::{
  timeout,
  tokio::{self, io::AsyncReadExt, net::TcpStream},
};

use super::RendezvousServer;

impl RendezvousServer {
  pub(super) async fn handle_port_listener(
    &self,
    stream: TcpStream,
    addr: SocketAddr,
  ) {
    let mut rs = self.clone();
    if addr.ip().is_loopback() {
      tokio::spawn(async move {
        let mut stream = stream;
        let mut buffer = [0; 1024];
        if let Ok(Ok(n)) = timeout(1000, stream.read(&mut buffer[..])).await {
          if let Ok(data) = std::str::from_utf8(&buffer[..n]) {
            // check command
          }
        }
      });
    }
  }
}
