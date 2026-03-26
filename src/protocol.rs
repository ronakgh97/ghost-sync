use crate::types::Result;
use bytes::Bytes;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use wincode::config::DefaultConfig;
use wincode::SchemaWrite;

/// Write a length-prefixed wincode-serialized frame.
#[inline]
pub async fn write_frame<W, T>(writer: &mut W, msg: &T) -> Result<()>
where
    W: AsyncWrite + Unpin,
    T: SchemaWrite<DefaultConfig, Src = T> + ?Sized,
{
    let payload = wincode::serialize(msg)
        .map_err(|e| crate::types::SyncError::Protocol(format!("serialize failed: {:?}", e)))?;
    let len = payload.len() as u32;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(&payload).await?;
    writer.flush().await?;
    Ok(())
}

/// Read a length-prefixed wincode-deserialized frame with a max payload size check.
#[inline]
pub async fn read_frame_raw<R>(reader: &mut R, max_payload: usize) -> Result<Bytes>
where
    R: AsyncRead + Unpin,
{
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    if len > max_payload {
        return Err(crate::types::SyncError::PayloadTooLarge {
            size: len,
            max: max_payload,
        });
    }

    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await?;
    Ok(Bytes::from(payload))
}

/// Write a raw byte slice as a length-prefixed frame to tcp buffer
/// Flushes on every writes btw
#[inline]
pub async fn write_frame_raw<W>(writer: &mut W, payload: Bytes) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let len = payload.len() as u32;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(&payload).await?;
    writer.flush().await?;
    Ok(())
}
