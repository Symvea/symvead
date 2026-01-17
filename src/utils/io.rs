use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Read exactly `len` bytes or fail
pub async fn read_exact<R: AsyncRead + Unpin>(
    reader: &mut R,
    len: usize,
) -> std::io::Result<Vec<u8>> {
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;
    Ok(buf)
}

/// Write all bytes or fail
pub async fn write_all<W: AsyncWrite + Unpin>(
    writer: &mut W,
    buf: &[u8],
) -> std::io::Result<()> {
    writer.write_all(buf).await?;
    writer.flush().await?;
    Ok(())
}
