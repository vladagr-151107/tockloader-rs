// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OXIDOS AUTOMOTIVE 2024.

/// Attributes are key-value pairs that describe hardware configuration, stored
/// in a fixed 64-byte format:
///
/// 1. Bytes 0–7: UTF-8 key string with null-byte padding for shorter strings
/// 2. Byte 8: Value length (Must be between 1 and 55)
/// 3. Bytes 9–63: UTF-8 value string. Null-padded to length.
///
/// See also <https://book.tockos.org/doc/kernel_attributes.html?highlight=attributes#header-format>
#[derive(Debug)]
pub struct DecodedAttribute {
    pub key: String,
    pub value: String,
}

impl DecodedAttribute {
    /// Total size of one encoded attribute is: 8 byte key + 1 byte value
    /// length + up to 55 byte value
    pub(crate) const ENCODED_LEN: usize = 64;
    /// Maximum allowed length of an attribute's value field: 55 bytes remain
    /// (bytes 9–63) after the 8-byte key and 1-byte length fields.
    const ATTRIBUTE_LEN: u8 = 55;
    pub(crate) fn new(decoded_key: String, decoded_value: String) -> DecodedAttribute {
        DecodedAttribute {
            key: decoded_key,
            value: decoded_value,
        }
    }

    /// Decode raw data into a [DecodedAttribute].
    ///
    /// # Params
    /// - `encoded` - byte array of at least 64 bytes.
    ///
    /// # Returns
    /// - `None` for an invalid property (invalid value length or invalid utf-8
    ///   data).
    /// - `Some(_)` otherwise
    pub fn decode(encoded: &[u8]) -> Option<DecodedAttribute> {
        let raw_key = &encoded[0..8];

        let key = std::str::from_utf8(raw_key)
            .ok()?
            .trim_end_matches('\0')
            .to_string();

        let vlen = encoded[8];

        if vlen > DecodedAttribute::ATTRIBUTE_LEN || vlen == 0 {
            return None;
        }

        let raw_value = &encoded[9..(9 + vlen as usize)];

        let value = std::str::from_utf8(raw_value)
            .ok()?
            .trim_end_matches('\0')
            .to_string();

        Some(DecodedAttribute::new(key, value))
    }
}
