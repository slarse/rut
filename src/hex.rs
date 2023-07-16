use std::num::ParseIntError;

pub fn to_hex_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{:x}", byte))
        .collect::<Vec<String>>()
        .join("")
}

pub fn from_hex_string(hex: &str) -> Result<Vec<u8>, ParseIntError> {
    hex.chars()
        .map(|chr| u8::from_str_radix(&chr.to_string(), 16))
        .collect()
}

pub fn from_hex_bytes(hex: &[u8]) -> Result<Vec<u8>, ParseIntError> {
    hex.iter()
        .map(|hex_byte| u8::from_str_radix(&hex_byte.to_string(), 16))
        .collect()
}

pub fn unhexlify(bytes: &[u8]) -> Vec<u8> {
    let mut unhexlified = Vec::new();
    for i in 0..bytes.len() {
        let compressed_bytes = bytes.get(i).unwrap();
        let left_byte = compressed_bytes >> 4;
        let right_byte = compressed_bytes & 0b00001111;
        unhexlified.push(left_byte);
        unhexlified.push(right_byte);
    }
    unhexlified
}

pub fn hexlify(bytes: &[u8]) -> Vec<u8> {
    let mut hexlified = Vec::new();
    for i in (0..bytes.len() - 1).step_by(2) {
        let left_byte = bytes.get(i).unwrap();
        let right_byte = bytes.get(i + 1).unwrap();
        let compressed = (left_byte << 4) | right_byte;
        hexlified.push(compressed);
    }

    hexlified
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;

    #[test]
    fn hexlify_and_unhexlify_roundtrip_works() {
        let bytes = vec![0, 1, 2, 3, 4, 6];

        let hexlified = hexlify(&bytes);
        let unhexlified = unhexlify(&hexlified);

        assert_vectors_equal(&unhexlified.to_vec(), &bytes)
    }

    fn assert_vectors_equal<T: Debug + Eq>(actual: &Vec<T>, expected: &Vec<T>) {
        if actual.len() != expected.len() {
            panic!(
                "expected vector has length {}, but actual vector has length {}",
                expected.len(),
                actual.len()
            );
        }

        for (actual, expected) in actual.iter().zip(expected.iter()) {
            if actual != expected {
                panic!(
                    "mismatching characters, expected={:?}, actual={:?}",
                    expected, actual
                );
            }
        }
    }
}
