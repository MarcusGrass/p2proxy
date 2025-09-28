use crate::display_chain;
use anyhow::Context;
use iroh::endpoint::{ConnectionError, ReadError, RecvStream, SendStream, VarInt, WriteError};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{ReadHalf, WriteHalf};

pub struct BufferedCopy<const N: usize> {
    read_offset: usize,
    write_offset: usize,
    data: Box<[u8; N]>,
}

#[derive(Debug, thiserror::Error)]
pub enum BufCopyError {
    #[error("Quic connection forbidden")]
    QuicConnectionForbidden,
    #[error("Quic forbidden")]
    QuicStreamForbidden,
    #[error("Quic internal")]
    QuicInternal,
    #[error("Quic closed")]
    QuicClosed(u64),
    #[error("Tcp EOF")]
    TCPEoF,
    #[error(transparent)]
    Unactionable(#[from] anyhow::Error),
}

impl BufCopyError {
    fn from_varint(var_int: VarInt) -> Self {
        match var_int {
            crate::proto::QUIC_OK_ERROR_CODE => Self::QuicClosed(var_int.into_inner()),
            crate::proto::GENERIC_QUIC_ERROR_CODE => Self::QuicInternal,
            crate::proto::FORBIDDEN_QUIC_ERROR_CODE => Self::QuicStreamForbidden,
            unk => Self::Unactionable(anyhow::anyhow!(
                "quic stream stopped with unmapped code: {unk}",
            )),
        }
    }
}

pub trait TcpOrQuicWrite {
    fn write(&mut self, buf: &[u8]) -> impl Future<Output = Result<usize, BufCopyError>> + Send;
}

impl TcpOrQuicWrite for WriteHalf<'_> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, BufCopyError> {
        AsyncWriteExt::write(self, buf)
            .await
            .context("failed to write to TCP")
            .map_err(BufCopyError::from)
    }
}

impl TcpOrQuicWrite for SendStream {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, BufCopyError> {
        let res = SendStream::write(self, buf).await;
        match res {
            Ok(o) => Ok(o),
            Err(WriteError::Stopped(e)) => Err(BufCopyError::from_varint(e)),
            Err(WriteError::ConnectionLost(ConnectionError::ApplicationClosed(cc))) => {
                if cc.error_code == crate::proto::FORBIDDEN_QUIC_ERROR_CODE {
                    Err(BufCopyError::QuicConnectionForbidden)
                } else {
                    Err(BufCopyError::QuicClosed(cc.error_code.into_inner()))
                }
            }
            Err(e) => Err(BufCopyError::Unactionable(anyhow::anyhow!(
                "write error: {}",
                display_chain(&e)
            ))),
        }
    }
}

pub trait TcpOrQuicRead {
    fn read(&mut self, buf: &mut [u8]) -> impl Future<Output = Result<usize, BufCopyError>>;
}

impl TcpOrQuicRead for ReadHalf<'_> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, BufCopyError> {
        let bytes = AsyncReadExt::read(self, buf)
            .await
            .context("failed to read from TCP")
            .map_err(BufCopyError::from)?;
        if bytes == 0 {
            return Err(BufCopyError::TCPEoF);
        }
        Ok(bytes)
    }
}

impl TcpOrQuicRead for RecvStream {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, BufCopyError> {
        match RecvStream::read(self, buf).await {
            Ok(Some(bytes)) => Ok(bytes),
            Ok(None) => Err(BufCopyError::Unactionable(anyhow::anyhow!("quic EOF"))),
            Err(ReadError::ConnectionLost(ConnectionError::ApplicationClosed(cc))) => {
                if cc.error_code == crate::proto::FORBIDDEN_QUIC_ERROR_CODE {
                    Err(BufCopyError::QuicConnectionForbidden)
                } else {
                    Err(BufCopyError::QuicClosed(cc.error_code.into_inner()))
                }
            }
            Err(ReadError::Reset(code)) => Err(BufCopyError::from_varint(code)),
            Err(e) => Err(BufCopyError::Unactionable(anyhow::anyhow!(
                "read error: {}",
                display_chain(&e)
            ))),
        }
    }
}

impl<const N: usize> BufferedCopy<N> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            read_offset: 0,
            write_offset: 0,
            data: Box::new([0; N]),
        }
    }

    // Cancel safe copy
    pub async fn copy(
        &mut self,
        input: &mut impl TcpOrQuicRead,
        output: &mut impl TcpOrQuicWrite,
    ) -> Result<(), BufCopyError> {
        loop {
            let needs_copy = self.write_offset - self.read_offset;
            if needs_copy > 0 {
                let sect = &self.data[self.read_offset..self.write_offset];
                let written = output.write(sect).await?;
                if written == 0 && !sect.is_empty() {
                    return Err(BufCopyError::Unactionable(anyhow::anyhow!(
                        "failed to write, write end closed"
                    )));
                }
                self.read_offset += written;
                if self.read_offset == self.write_offset {
                    self.read_offset = 0;
                    self.write_offset = 0;
                } else {
                    let rem = self.write_offset - self.read_offset;
                    self.data
                        .copy_within(self.read_offset..self.write_offset, 0);
                    self.read_offset = 0;
                    self.write_offset = rem;
                }
            }
            self.read_bytes(input).await?;
        }
    }

    async fn read_bytes(&mut self, input: &mut impl TcpOrQuicRead) -> Result<(), BufCopyError> {
        let sect = &mut self.data[self.write_offset..];
        let read_bytes = input.read(sect).await?;
        self.write_offset += read_bytes;
        Ok(())
    }
}
