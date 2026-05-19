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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{duplex, AsyncWriteExt};

    fn chat_msg() -> Frame {
        Frame::ChatMsg {
            id: "test-id".into(),
            from: "peer1".into(),
            from_name: "test-host".into(),
            from_ip: "192.168.1.100".into(),
            from_os: "linux".into(),
            content: "Hello, world!".into(),
            to: "*".into(),
            created_at: "2025-01-01T00:00:00Z".into(),
        }
    }

    #[tokio::test]
    async fn test_write_then_read_chat_msg() {
        let (mut a, mut b) = duplex(4096);

        let frame = chat_msg();

        // Write on side a
        write_frame(&mut a, &frame).await.unwrap();
        a.shutdown().await.unwrap(); // signal EOF so read_frame sees end

        // Read on side b
        let read = read_frame(&mut b).await.unwrap();
        assert!(read.is_some());
        match read.unwrap() {
            Frame::ChatMsg { content, from, to, .. } => {
                assert_eq!(content, "Hello, world!");
                assert_eq!(from, "peer1");
                assert_eq!(to, "*");
            }
            other => panic!("Expected ChatMsg, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_hello_roundtrip() {
        let (mut a, mut b) = duplex(256);

        let hello = Frame::Hello {
            id: "my-host-abc1".into(),
        };
        write_frame(&mut a, &hello).await.unwrap();
        a.shutdown().await.unwrap();

        let read = read_frame(&mut b).await.unwrap();
        assert!(read.is_some());
        match read.unwrap() {
            Frame::Hello { id } => assert_eq!(id, "my-host-abc1"),
            other => panic!("Expected Hello, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_ping_pong() {
        let (mut a, mut b) = duplex(256);

        write_frame(&mut a, &Frame::Ping).await.unwrap();
        a.shutdown().await.unwrap();

        let read = read_frame(&mut b).await.unwrap();
        assert!(read.is_some());
        match read.unwrap() {
            Frame::Ping => {} // ok
            other => panic!("Expected Ping, got {:?}", other),
        }

        // Write Pong back
        drop(a);
        let (mut a2, mut b2) = duplex(256);
        write_frame(&mut a2, &Frame::Pong).await.unwrap();
        let read2 = read_frame(&mut b2).await.unwrap();
        assert!(read2.is_some());
        match read2.unwrap() {
            Frame::Pong => {} // ok
            other => panic!("Expected Pong, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_eof_returns_none() {
        let mut buf = tokio::io::BufReader::new(tokio::io::empty());
        let result = read_frame(&mut buf).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_empty_frame_rejected() {
        let (mut a, mut b) = duplex(256);
        // Write length = 0 (invalid)
        a.write_all(&[0u8; 4]).await.unwrap();
        a.shutdown().await.unwrap();

        let result = read_frame(&mut b).await;
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Invalid frame size"));
    }

    #[tokio::test]
    async fn test_frame_too_large_rejected() {
        let frame = Frame::ChatMsg {
            id: "x".repeat(MAX_FRAME_SIZE / 2), // will exceed MAX_FRAME_SIZE when serialized
            from: String::new(),
            from_name: String::new(),
            from_ip: String::new(),
            from_os: String::new(),
            content: "x".repeat(MAX_FRAME_SIZE / 2),
            to: String::new(),
            created_at: String::new(),
        };
        let result = write_frame(&mut tokio::io::sink(), &frame).await;
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Frame too large"));
    }

    #[tokio::test]
    async fn test_garbage_json_rejected() {
        let mut buf = Vec::new();
        let data = b"{invalid json}";
        let len = (data.len() as u32).to_be_bytes();
        buf.extend_from_slice(&len);
        buf.extend_from_slice(data);

        let result = read_frame(&mut tokio::io::BufReader::new(&buf[..])).await;
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Deserialize error"));
    }

    #[tokio::test]
    async fn test_system_frame() {
        let (mut a, mut b) = duplex(256);

        write_frame(&mut a, &Frame::System { content: "test system message".into() }).await.unwrap();
        a.shutdown().await.unwrap();

        let read = read_frame(&mut b).await.unwrap();
        assert!(read.is_some());
        match read.unwrap() {
            Frame::System { content } => assert_eq!(content, "test system message"),
            other => panic!("Expected System, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_long_content_roundtrip() {
        let (mut a, mut b) = duplex(65536);

        let long_content = "A".repeat(10000);
        let frame = Frame::ChatMsg {
            id: "id-1".into(),
            from: "peer-b".into(),
            from_name: "remote".into(),
            from_ip: "10.0.0.1".into(),
            from_os: "windows".into(),
            content: long_content.clone(),
            to: "peer-a".into(),
            created_at: "2025-06-01T12:00:00+00:00".into(),
        };

        write_frame(&mut a, &frame).await.unwrap();
        a.shutdown().await.unwrap();

        let read = read_frame(&mut b).await.unwrap();
        assert!(read.is_some());
        match read.unwrap() {
            Frame::ChatMsg { content, .. } => {
                assert_eq!(content.len(), 10000);
                assert_eq!(content, long_content);
            }
            other => panic!("Expected ChatMsg, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_special_characters_in_message() {
        let (mut a, mut b) = duplex(4096);

        let special = "Hello\nNew Line\tTab\nUnicode: \\u{1F600} \u{1F600} Emoji!";
        let frame = Frame::ChatMsg {
            id: "spec".into(),
            from: "p1".into(),
            from_name: "host".into(),
            from_ip: "::1".into(),
            from_os: "macos".into(),
            content: special.into(),
            to: "*".into(),
            created_at: "2025-01-01T00:00:00Z".into(),
        };

        write_frame(&mut a, &frame).await.unwrap();
        a.shutdown().await.unwrap();

        let read = read_frame(&mut b).await.unwrap();
        assert!(read.is_some());
        match read.unwrap() {
            Frame::ChatMsg { content, .. } => assert_eq!(content, special),
            other => panic!("Expected ChatMsg, got {:?}", other),
        }
    }
}
