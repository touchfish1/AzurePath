use crate::types::chat::Frame;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024; // 16 MB

pub async fn write_frame(
    writer: &mut (impl AsyncWrite + Unpin),
    frame: &Frame,
) -> Result<(), String> {
    let data = serde_json::to_vec(frame).map_err(|e| format!("Serialize error: {}", e))?;
    if data.len() > MAX_FRAME_SIZE {
        return Err("Frame too large".to_string());
    }
    let len = (data.len() as u32).to_be_bytes();
    writer
        .write_all(&len)
        .await
        .map_err(|e| format!("Write length error: {}", e))?;
    writer
        .write_all(&data)
        .await
        .map_err(|e| format!("Write data error: {}", e))?;
    Ok(())
}

pub async fn read_frame(
    reader: &mut (impl AsyncRead + Unpin),
) -> Result<Option<Frame>, String> {
    let mut len_buf = [0u8; 4];
    match reader.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(format!("Read length error: {}", e)),
    }

    let len = u32::from_be_bytes(len_buf) as usize;
    if len == 0 || len > MAX_FRAME_SIZE {
        return Err(format!("Invalid frame size: {}", len));
    }

    let mut data = vec![0u8; len];
    reader
        .read_exact(&mut data)
        .await
        .map_err(|e| format!("Read data error: {}", e))?;

    let frame: Frame =
        serde_json::from_slice(&data).map_err(|e| format!("Deserialize error: {}", e))?;
    Ok(Some(frame))
}
