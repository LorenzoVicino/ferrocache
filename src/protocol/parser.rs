use std::str;

use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt};

use super::Frame;

pub async fn read_frame<R>(reader: &mut R) -> std::io::Result<Option<Frame>>
where
    R: AsyncBufRead + Unpin,
{
    let mut line = Vec::new();
    let bytes_read = reader.read_until(b'\n', &mut line).await?;

    if bytes_read == 0 {
        return Ok(None);
    }

    let line = trim_crlf(&line);

    if line.is_empty() {
        return Ok(Some(Frame::Error("ERR empty request".to_string())));
    }

    match line[0] {
        b'*' => read_array(reader, &line[1..]).await.map(Some),
        b'$' => read_bulk(reader, &line[1..]).await.map(Some),
        b'+' => Ok(Some(Frame::Simple(bytes_to_string(&line[1..])?))),
        b':' => Ok(Some(Frame::Integer(parse_i64(&line[1..])?))),
        _ => Ok(Some(parse_inline_command(line))),
    }
}

async fn read_array<R>(reader: &mut R, len_bytes: &[u8]) -> std::io::Result<Frame>
where
    R: AsyncBufRead + Unpin,
{
    let len = parse_usize(len_bytes)?;
    let mut items = Vec::with_capacity(len);

    for _ in 0..len {
        match Box::pin(read_frame(reader)).await? {
            Some(frame) => items.push(frame),
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "unexpected EOF while reading RESP array",
                ))
            }
        }
    }

    Ok(Frame::Array(items))
}

async fn read_bulk<R>(reader: &mut R, len_bytes: &[u8]) -> std::io::Result<Frame>
where
    R: AsyncBufRead + Unpin,
{
    let len = parse_i64(len_bytes)?;

    if len == -1 {
        return Ok(Frame::Null);
    }

    if len < 0 {
        return Err(invalid_data("negative bulk length"));
    }

    let len = len as usize;
    let mut buffer = vec![0; len + 2];
    reader.read_exact(&mut buffer).await?;

    if &buffer[len..] != b"\r\n" {
        return Err(invalid_data("bulk string missing CRLF terminator"));
    }

    buffer.truncate(len);
    Ok(Frame::Bulk(buffer))
}

fn parse_inline_command(line: &[u8]) -> Frame {
    let args = line
        .split(|byte| byte.is_ascii_whitespace())
        .filter(|arg| !arg.is_empty())
        .map(|arg| Frame::Bulk(arg.to_vec()))
        .collect();

    Frame::Array(args)
}

fn trim_crlf(line: &[u8]) -> &[u8] {
    line.strip_suffix(b"\r\n")
        .or_else(|| line.strip_suffix(b"\n"))
        .unwrap_or(line)
}

fn parse_usize(bytes: &[u8]) -> std::io::Result<usize> {
    let value = bytes_to_string(bytes)?;
    value.parse().map_err(|_| invalid_data("invalid integer"))
}

fn parse_i64(bytes: &[u8]) -> std::io::Result<i64> {
    let value = bytes_to_string(bytes)?;
    value.parse().map_err(|_| invalid_data("invalid integer"))
}

fn bytes_to_string(bytes: &[u8]) -> std::io::Result<String> {
    str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| invalid_data("invalid UTF-8"))
}

fn invalid_data(message: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use tokio::io::BufReader;

    use super::*;

    #[tokio::test]
    async fn reads_resp_array_command() {
        let input = Cursor::new(b"*2\r\n$4\r\nECHO\r\n$5\r\nhello\r\n".to_vec());
        let mut input = BufReader::new(input);

        let frame = read_frame(&mut input).await.unwrap().unwrap();

        assert_eq!(
            frame,
            Frame::Array(vec![
                Frame::Bulk(b"ECHO".to_vec()),
                Frame::Bulk(b"hello".to_vec())
            ])
        );
    }

    #[tokio::test]
    async fn reads_inline_command() {
        let input = Cursor::new(b"PING hello\r\n".to_vec());
        let mut input = BufReader::new(input);

        let frame = read_frame(&mut input).await.unwrap().unwrap();

        assert_eq!(
            frame,
            Frame::Array(vec![
                Frame::Bulk(b"PING".to_vec()),
                Frame::Bulk(b"hello".to_vec())
            ])
        );
    }
}
