//! Strict browser-native messaging frames, independent of the browser process.
use anyhow::{Result, ensure};
use serde::Deserialize;
use std::io::{Read, Write};
pub const MAX: usize = 64 * 1024;
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub action: String,
    pub url: Option<String>,
    pub name: Option<String>,
    pub size: Option<u64>,
    pub mime: Option<String>,
    pub id: Option<String>,
    pub path: Option<std::path::PathBuf>,
}
pub fn read_message(input: &mut impl Read) -> Result<Option<Message>> {
    let mut prefix = [0u8; 4];
    let first = loop {
        match input.read(&mut prefix[..1]) {
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            result => break result?,
        }
    };
    if first == 0 {
        return Ok(None);
    }
    input.read_exact(&mut prefix[1..])?;
    let length = u32::from_le_bytes(prefix) as usize;
    ensure!(length <= MAX, "Native message too large");
    let mut bytes = vec![0; length];
    input.read_exact(&mut bytes)?;
    Ok(Some(serde_json::from_slice(&bytes)?))
}
pub fn write_message(output: &mut impl Write, value: &serde_json::Value) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    ensure!(bytes.len() <= MAX, "Native response too large");
    output.write_all(&(bytes.len() as u32).to_le_bytes())?;
    output.write_all(&bytes)?;
    output.flush()?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_native_frames_and_clean_eof() {
        let mut bytes = vec![];
        write_message(&mut bytes, &serde_json::json!({"action":"ping"})).unwrap();
        let mut input = std::io::Cursor::new(bytes);
        assert_eq!(read_message(&mut input).unwrap().unwrap().action, "ping");
        assert!(read_message(&mut input).unwrap().is_none());
    }
    #[test]
    fn oversized_truncated_and_unknown_fields_rejected() {
        assert!(read_message(&mut std::io::Cursor::new((MAX as u32 + 1).to_le_bytes())).is_err());
        assert!(read_message(&mut std::io::Cursor::new(vec![1, 0])).is_err());
        let mut bytes = vec![];
        write_message(
            &mut bytes,
            &serde_json::json!({"action":"add","url":"https://example.org","cookie":"secret"}),
        )
        .unwrap();
        assert!(read_message(&mut std::io::Cursor::new(bytes)).is_err());
    }
}
#[cfg(test)]
mod interrupted_reads {
    use super::*;
    struct Reader {
        first: bool,
        bytes: std::io::Cursor<Vec<u8>>,
    }
    impl Read for Reader {
        fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
            if self.first {
                self.first = false;
                return Err(std::io::ErrorKind::Interrupted.into());
            }
            self.bytes.read(bytes)
        }
    }
    #[test]
    fn interrupted_prefix_reads_preserve_the_native_connection() {
        let mut bytes = vec![];
        write_message(&mut bytes, &serde_json::json!({"action":"ping"})).unwrap();
        let mut input = Reader {
            first: true,
            bytes: std::io::Cursor::new(bytes),
        };
        assert_eq!(read_message(&mut input).unwrap().unwrap().action, "ping");
        assert!(read_message(&mut input).unwrap().is_none());
    }
}
