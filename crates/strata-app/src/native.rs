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
}
pub fn read_message(input: &mut impl Read) -> Result<Option<Message>> {
    let mut prefix = [0u8; 4];
    let first = input.read(&mut prefix[..1])?;
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
