use tokio::io::{AsyncWrite, AsyncWriteExt};

use super::Frame;

pub async fn write_frame<W>(writer: &mut W, frame: &Frame) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    match frame {
        Frame::Simple(value) => {
            writer.write_all(b"+").await?;
            writer.write_all(value.as_bytes()).await?;
            writer.write_all(b"\r\n").await?;
        }
        Frame::Error(value) => {
            writer.write_all(b"-").await?;
            writer.write_all(value.as_bytes()).await?;
            writer.write_all(b"\r\n").await?;
        }
        Frame::Integer(value) => {
            writer.write_all(format!(":{value}\r\n").as_bytes()).await?;
        }
        Frame::Bulk(value) => {
            writer
                .write_all(format!("${}\r\n", value.len()).as_bytes())
                .await?;
            writer.write_all(value).await?;
            writer.write_all(b"\r\n").await?;
        }
        Frame::Null => {
            writer.write_all(b"$-1\r\n").await?;
        }
        Frame::Array(items) => {
            writer
                .write_all(format!("*{}\r\n", items.len()).as_bytes())
                .await?;
            for item in items {
                Box::pin(write_frame(writer, item)).await?;
            }
        }
    }

    writer.flush().await
}

