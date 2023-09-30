use std::io::Read;

use anyhow::Result;

// Function to split Vec<u8> into lines
fn split_lines(buffer: Vec<u8>) -> Vec<Vec<u8>> {
    buffer
        .split(|x: &u8| *x == b'\n' || *x == 0x0D || *x == 0x0A) // '\n', '\r'
        .filter(|x| !x.is_empty())
        .map(|x| x.to_vec())
        .collect()
}

// Function to read File and split lines
pub fn read_lines<T: Read>(file: &mut T) -> Result<Vec<Vec<u8>>> {
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(split_lines(buffer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_lines() {
        let buffer = b"line1\nline2\r\nline3\rline4\n".to_vec();
        let result = split_lines(buffer);
        assert_eq!(
            result,
            vec![
                b"line1".to_vec(),
                b"line2".to_vec(),
                b"line3".to_vec(),
                b"line4".to_vec()
            ]
        );
    }
}
