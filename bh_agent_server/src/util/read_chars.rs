use std::fs::File;
use std::io::Read;

use anyhow::Result;

use bh_agent_common::FileOpenType;

pub fn read_generic(mut file: &File, n: u32, file_type: FileOpenType) -> Result<Vec<u8>> {
    match file_type {
        FileOpenType::Binary => {
            let mut buffer = vec![0u8; n as usize];
            file.read(&mut buffer)?;
            Ok(buffer)
        }
        FileOpenType::Text => Ok(read_chars(&mut file, n as usize)?),
    }
}

pub fn read_chars<R: Read>(reader: &mut R, n: usize) -> Result<Vec<u8>> {
    let mut buffer = vec![0u8; n];
    let mut result = String::new();

    let bytes_read = reader.read(&mut buffer)?;
    buffer.truncate(bytes_read); // Truncate buffer to actual bytes read

    while result.chars().count() < n && !buffer.is_empty() {
        let utf8_str = std::str::from_utf8(&buffer);

        match utf8_str {
            Ok(s) => {
                result.push_str(s);
                break;
            }
            Err(err) if err.valid_up_to() > 0 => {
                let valid_str = std::str::from_utf8(&buffer[0..err.valid_up_to()]).unwrap();
                result.push_str(valid_str);
                buffer.drain(0..err.valid_up_to());
            }
            _ => {}
        }

        if result.chars().count() < n {
            let mut additional_buffer = vec![0u8; n - result.chars().count()];
            let additional_bytes = reader.read(&mut additional_buffer)?;
            if additional_bytes == 0 {
                break;
            }
            buffer.extend_from_slice(&additional_buffer[0..additional_bytes]);
        }
    }

    Ok(result.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::io::Cursor;

    #[test]
    fn test_single_byte_chars() -> io::Result<()> {
        let data = "abcdef";
        let mut cursor = Cursor::new(data.as_bytes());
        let result = read_chars(&mut cursor, 3);
        assert_eq!(result.unwrap(), b"abc");
        Ok(())
    }

    #[test]
    fn test_multi_byte_chars() -> io::Result<()> {
        let data = "a😀b";
        let mut cursor = Cursor::new(data.as_bytes());
        let result = read_chars(&mut cursor, 2);
        assert_eq!(result.unwrap(), "a😀".as_bytes());
        Ok(())
    }

    #[test]
    fn test_mixed_chars() -> io::Result<()> {
        let data = "a😀b😂c";
        let mut cursor = Cursor::new(data.as_bytes());
        let result = read_chars(&mut cursor, 4);
        assert_eq!(result.unwrap(), "a😀b😂".as_bytes());
        Ok(())
    }
}
