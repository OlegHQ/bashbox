//! Shared gzip compress/decompress helpers for `gzip` and `tar` (flate2).

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::{Read, Write};

/// Default compression level (matches GNU gzip).
pub const DEFAULT_GZIP_LEVEL: u32 = 6;

/// Compress at [`DEFAULT_GZIP_LEVEL`] (shared by `tar -z` and callers that mirror GNU defaults).
pub fn compress_gzip_default(data: &[u8]) -> Result<Vec<u8>, String> {
    compress_gzip(data, DEFAULT_GZIP_LEVEL)
}

/// Compress raw bytes with gzip at the given compression level (1–9).
pub fn compress_gzip(data: &[u8], level: u32) -> Result<Vec<u8>, String> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::new(level));
    encoder.write_all(data).map_err(|e| e.to_string())?;
    encoder.finish().map_err(|e| e.to_string())
}

/// Decompress gzip-compressed bytes.
pub fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| e.to_string())?;
    Ok(decompressed)
}

/// True if `data` begins with gzip magic bytes (0x1f 0x8b).
pub fn is_gzip(data: &[u8]) -> bool {
    data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b
}
