use core::convert::TryFrom;
use tbf_parser::parse::*;
use tbf_parser::types;
use tbf_parser::types::Flags;

// Serialization

#[test]
fn serialize_identical_with_original() {
    let buffer: Vec<u8> = include_bytes!("./flashes/simple.dat").to_vec();

    let (_, header_len, _) = parse_tbf_header_lengths(&buffer[0..8].try_into().unwrap())
        .ok()
        .unwrap();

    let header = parse_tbf_header(&buffer[0..header_len as usize], 2).unwrap();
    let serialized = header.serialize();

    // Check if serialize matches original buffer
    assert_eq!(&buffer[0..16], &serialized[..]);
}

// Flag modifications
#[test]
fn flags_modifications() {
    let mut buffer = include_bytes!("./flashes/footerSHA256.dat").to_vec();
    let (_, header_len, _) = parse_tbf_header_lengths(&buffer[0..8].try_into().unwrap())
        .ok()
        .unwrap();

    let mut header = parse_tbf_header(&buffer[0..header_len as usize], 2).unwrap();
    assert!(header.enabled());
    header.set_sticky(true);
    // Set sticky without parsing
    assert!(header.sticky());
    // Unset
    header.set_sticky(false);

    // Disable
    header.set_enabled(false);
    let serialized = header.serialize();
    buffer[0..16].copy_from_slice(&serialized);

    let reparsed = parse_tbf_header(&buffer[0..header_len as usize], 2).unwrap();
    assert!(!reparsed.enabled());

    // Enable
    let mut header = reparsed;
    header.set_enabled(true);
    let serialized = header.serialize();
    buffer[0..16].copy_from_slice(&serialized);

    let reparsed = parse_tbf_header(&buffer[0..header_len as usize], 2).unwrap();
    assert!(reparsed.enabled());
}

#[test]
fn padding_header_set_flags() {
    let buffer = vec![
        0x02, 0x00, 0x10, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x12, 0x00, 0x10,
        0x00,
    ];

    let (_, header_len, _) = parse_tbf_header_lengths(&buffer[0..8].try_into().unwrap())
        .ok()
        .unwrap();
    let mut header = parse_tbf_header(&buffer[0..header_len as usize], 2).unwrap();

    assert!(!header.is_app());

    let flags = Flags::Enabled | Flags::Sticky;
    header.set_flags(flags);

    let serialized = header.serialize();
    let flags = u32::from_le_bytes(serialized[8..12].try_into().unwrap());
    assert_eq!(flags, (Flags::Enabled | Flags::Sticky).as_u32());
}

#[test]
fn fields_preserved() {
    let buffer = include_bytes!("./flashes/simple.dat").to_vec();
    let (_, header_len, _) = parse_tbf_header_lengths(&buffer[0..8].try_into().unwrap())
        .ok()
        .unwrap();

    let mut header = parse_tbf_header(&buffer[0..header_len as usize], 2).unwrap();

    let header_size = header.header_size();
    let total_size = header.total_size();

    header.set_flags(Flags::Enabled | Flags::Sticky);
    let serialized = header.serialize();

    let _version = u16::from_le_bytes(serialized[0..2].try_into().unwrap());
    let _header_size = u16::from_le_bytes(serialized[2..4].try_into().unwrap());
    let _total_size = u32::from_le_bytes(serialized[4..8].try_into().unwrap());

    // Check other fields are unchanged
    assert_eq!(_version, 2);
    assert_eq!(header_size, _header_size);
    assert_eq!(total_size, _total_size);
}

// Complete use //
#[test]
fn serialization_multiple_checks() {
    let mut buffer = include_bytes!("./flashes/footerRSA4096.dat").to_vec();
    let (_, header_len, _) = parse_tbf_header_lengths(&buffer[0..8].try_into().unwrap())
        .ok()
        .unwrap();

    let mut header = parse_tbf_header(&buffer[0..header_len as usize], 2).unwrap();
    assert!(header.enabled());

    // Disable
    header.set_enabled(false);
    let serialized = header.serialize();
    buffer[0..16].copy_from_slice(&serialized);

    let header = parse_tbf_header(&buffer[0..header_len as usize], 2).unwrap();
    assert!(!header.enabled());

    // Enable and set sticky
    let mut header = header;
    header.set_enabled(true);
    header.set_sticky(true);
    let serialized = header.serialize();
    buffer[0..16].copy_from_slice(&serialized);

    let header = parse_tbf_header(&buffer[0..header_len as usize], 2).unwrap();
    assert!(header.enabled());
    assert!(header.sticky());

    // Disable sticky with high bits
    let flags = !Flags::Sticky;
    let mut header = header;
    header.set_flags(flags);
    let serialized = header.serialize();
    buffer[0..16].copy_from_slice(&serialized);

    let header = parse_tbf_header(&buffer[0..header_len as usize], 2).unwrap();
    assert!(header.enabled());
    assert!(!header.sticky());
    let flags_buffer = u32::from_le_bytes(buffer[8..12].try_into().unwrap());
    assert_eq!(flags_buffer, (!Flags::Sticky).as_u32());
}

#[test]
fn corrupt() {
    let mut buffer = include_bytes!("./flashes/simple.dat").to_vec();
    let (_, header_len, _) = parse_tbf_header_lengths(&buffer[0..8].try_into().unwrap())
        .ok()
        .unwrap();

    // Corrupt the checksum manually and check for parsing error
    buffer[12] ^= 0x6D;

    let result = parse_tbf_header(&buffer[0..header_len as usize], 2);
    assert!(result.is_err());
}

#[test]
fn main_tlv_matches_fixture() {
    let buffer = include_bytes!("./flashes/simple.dat");
    let mut out = [0u8; 16];
    let main = types::TbfHeaderV2Main::try_from(&buffer[20..32]).unwrap();
    types::serialize_main(&main, &mut out, 0);
    assert_eq!(&out[0..16], &buffer[16..32]);
}
