//! Framing of model files: a magic number, the length of a JSON header as a little-endian `u32`,
//! the header, and the weights of the network.
//!
//! The header and the weights are read by `cgt_ai_model`. The framing lives here so that tools
//! without a network backend, such as the website builder, can still look into a model file.

const MAGIC: &[u8; 8] = b"cgt-ai\0\x01";

pub fn join(header: &[u8], weights: &[u8]) -> Vec<u8> {
    let len = u32::try_from(header.len()).expect("the header is shorter than 4 GiB");
    let mut bytes = Vec::with_capacity(MAGIC.len() + 4 + header.len() + weights.len());
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&len.to_le_bytes());
    bytes.extend_from_slice(header);
    bytes.extend_from_slice(weights);
    bytes
}

/// The header and the weights of a model file, `None` if `bytes` are not a model file.
pub fn split(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    let (len, rest) = bytes.strip_prefix(MAGIC)?.split_first_chunk::<4>()?;
    let len = u32::from_le_bytes(*len) as usize;
    (len <= rest.len()).then(|| rest.split_at(len))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let bytes = join(b"{}", b"weights");
        assert_eq!(split(&bytes), Some((&b"{}"[..], &b"weights"[..])));
        assert_eq!(
            split(&bytes[..bytes.len() - 7]),
            Some((&b"{}"[..], &[][..]))
        );
        assert_eq!(split(&bytes[..13]), None);
        assert_eq!(split(b"weights"), None);
    }
}
