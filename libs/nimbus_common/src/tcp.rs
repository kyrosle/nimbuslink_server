use tokio::io::{AsyncRead, AsyncWrite};

pub trait TcpStreamTrait: AsyncRead + AsyncWrite + Unpin {}
pub struct DynTcpStream(Box<dyn TcpStreamTrait + Send + Sync>);
