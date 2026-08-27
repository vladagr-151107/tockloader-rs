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
    let (serialized, len) = header.serialize();

    // Check if serialize matches original buffer
    assert_eq!(&buffer[0..header_len as usize], &serialized[..len]);
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
    let (serialized, _len) = header.serialize();
    buffer[0..16].copy_from_slice(&serialized[..16]);

    let reparsed = parse_tbf_header(&buffer[0..header_len as usize], 2).unwrap();
    assert!(!reparsed.enabled());

    // Enable
    let mut header = reparsed;
    header.set_enabled(true);
    let (serialized, _len) = header.serialize();
    buffer[0..16].copy_from_slice(&serialized[..16]);

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

    let (serialized, _len) = header.serialize();
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
    let (serialized, _len) = header.serialize();

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
    let (serialized, _len) = header.serialize();
    buffer[0..16].copy_from_slice(&serialized[..16]);

    let header = parse_tbf_header(&buffer[0..header_len as usize], 2).unwrap();
    assert!(!header.enabled());

    // Enable and set sticky
    let mut header = header;
    header.set_enabled(true);
    header.set_sticky(true);
    let (serialized, _len) = header.serialize();
    buffer[0..16].copy_from_slice(&serialized[..16]);

    let header = parse_tbf_header(&buffer[0..header_len as usize], 2).unwrap();
    assert!(header.enabled());
    assert!(header.sticky());

    // Disable sticky with high bits
    let flags = !Flags::Sticky;
    let mut header = header;
    header.set_flags(flags);
    let (serialized, _len) = header.serialize();
    buffer[0..16].copy_from_slice(&serialized[..16]);

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

#[test]
fn program_tlv_matches_fixture() {
    let buffer = include_bytes!("./flashes/footerSHA256.dat");
    let mut out = [0u8; 24];
    let program = types::TbfHeaderV2Program::try_from(&buffer[36..56]).unwrap();
    types::serialize_program(&program, &mut out, 0);
    assert_eq!(&out[0..24], &buffer[32..56]);
}

#[test]
fn kernel_version_tlv_matches_fixture() {
    let buffer = include_bytes!("./flashes/simple.dat");
    let mut out = [0u8; 8];
    let kernel_version = types::TbfHeaderV2KernelVersion::try_from(&buffer[48..52]).unwrap();
    types::serialize_kernel_version(&kernel_version, &mut out, 0);
    assert_eq!(&out[0..8], &buffer[44..52]);
}

#[test]
fn fixed_addresses_tlv_round_trips() {
    let mut out = [0u8; 12];
    let bytes: [u8; 8] = [0x00, 0x10, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00];
    let fixed_addresses = types::TbfHeaderV2FixedAddresses::try_from(&bytes[..]).unwrap();
    types::serialize_fixed_addresses(&fixed_addresses, &mut out, 0);
    assert_eq!(&out[0..2], &5u16.to_le_bytes());
    assert_eq!(&out[2..4], &8u16.to_le_bytes());
    assert_eq!(&out[4..8], &0x1000u32.to_le_bytes());
    assert_eq!(&out[8..12], &0x2000u32.to_le_bytes());
}

#[test]
fn package_name_tlv_matches_fixture() {
    let buffer = include_bytes!("./flashes/simple.dat");
    let mut out = [0u8; 12];
    let name = types::TbfHeaderV2PackageName::<64>::try_from(&buffer[36..42]).unwrap();
    types::serialize_package_name(&name, &mut out, 0);
    assert_eq!(&out[0..12], &buffer[32..44]);
}

#[test]
fn short_id_tlv_round_trips() {
    let mut out = [0u8; 8];
    let short_id = types::TbfHeaderV2ShortId::try_from(&42u32.to_le_bytes()[..]).unwrap();

    types::serialize_short_id(&short_id, &mut out, 0);

    assert_eq!(&out[0..2], &10u16.to_le_bytes());
    assert_eq!(&out[2..4], &4u16.to_le_bytes());
    assert_eq!(&out[4..8], &42u32.to_le_bytes());
}

#[test]
fn writeable_regions_tlv_round_trips() {
    let mut out = [0u8; 20];
    let region1 = types::TbfHeaderV2WriteableFlashRegion::try_from(
        &[0x00, 0x10, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00][..],
    )
    .unwrap();
    let region2 = types::TbfHeaderV2WriteableFlashRegion::try_from(
        &[0x00, 0x20, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00][..],
    )
    .unwrap();
    let regions = [Some(region1), Some(region2), None, None];
    types::serialize_writeable_regions(&regions, &mut out, 0);

    assert_eq!(&out[0..2], &2u16.to_le_bytes());
    assert_eq!(&out[2..4], &16u16.to_le_bytes());
    assert_eq!(&out[4..8], &0x1000u32.to_le_bytes());
    assert_eq!(&out[8..12], &0x200u32.to_le_bytes());
    assert_eq!(&out[12..16], &0x2000u32.to_le_bytes());
    assert_eq!(&out[16..20], &0x400u32.to_le_bytes());
}

#[test]
fn storage_permissions_tlv_round_trips() {
    let mut out = [0u8; 20];

    let mut bytes = [0u8; 16];
    bytes[0..4].copy_from_slice(&7u32.to_le_bytes()); //write_id = 7
    bytes[4..6].copy_from_slice(&1u16.to_le_bytes()); // read_length = 1
    bytes[6..10].copy_from_slice(&42u32.to_le_bytes()); // read_ids[0] = 42
    bytes[10..12].copy_from_slice(&1u16.to_le_bytes()); // modify_length = 1
    bytes[12..16].copy_from_slice(&99u32.to_le_bytes()); // modify_ids[0] = 99

    let perms = types::TbfHeaderV2StoragePermissions::<8>::try_from(&bytes[..]).unwrap();

    types::serialize_storage_permissions(&perms, &mut out, 0);

    assert_eq!(&out[0..2], &7u16.to_le_bytes());
    assert_eq!(&out[2..4], &16u16.to_le_bytes());
    assert_eq!(&out[4..20], &bytes[..]);
}

#[test]
fn permissions_tlv_round_trips() {
    let mut out = [0u8; 24];

    let mut bytes = [0u8; 18];
    bytes[0..2].copy_from_slice(&1u16.to_le_bytes()); // length = 1
    bytes[2..6].copy_from_slice(&5u32.to_le_bytes()); // driver_number = 5
    bytes[6..10].copy_from_slice(&0u32.to_le_bytes()); // offset = 0
    bytes[10..18].copy_from_slice(&0xFFu64.to_le_bytes()); // allowed_commands = 0xFF

    let perms = types::TbfHeaderV2Permissions::<8>::try_from(&bytes[..]).unwrap();

    types::serialize_permissions(&perms, &mut out, 0);

    assert_eq!(&out[0..2], &6u16.to_le_bytes());
    assert_eq!(&out[2..4], &18u16.to_le_bytes());
    assert_eq!(&out[4..22.min(out.len())], &bytes[..]);
    assert_eq!(&out[22..24], &[0, 0]);
}

#[test]
fn full_header_matches_fixture() {
    let buffer = include_bytes!("./flashes/simple.dat");

    let (_, header_len, _) = parse_tbf_header_lengths(&buffer[0..8].try_into().unwrap())
        .ok()
        .unwrap();
    let header = parse_tbf_header(&buffer[0..header_len as usize], 2).unwrap();

    let (serialized, len) = header.serialize();

    assert_eq!(len, header_len as usize);
    assert_eq!(&serialized[0..len], &buffer[0..header_len as usize]);
}

#[test]
fn full_header_matches_footer_sha256_fixture() {
    let buffer = include_bytes!("./flashes/footerSHA256.dat");

    let (_, header_len, _) = parse_tbf_header_lengths(&buffer[0..8].try_into().unwrap())
        .ok()
        .unwrap();
    let header = parse_tbf_header(&buffer[0..header_len as usize], 2).unwrap();

    let (serialized, len) = header.serialize();

    assert_eq!(len, header_len as usize);
    assert_eq!(&serialized[0..len], &buffer[0..header_len as usize]);
}

#[test]
fn full_header_matches_footer_rsa4096_fixture() {
    let buffer = include_bytes!("./flashes/footerRSA4096.dat");

    let (_, header_len, _) = parse_tbf_header_lengths(&buffer[0..8].try_into().unwrap())
        .ok()
        .unwrap();
    let header = parse_tbf_header(&buffer[0..header_len as usize], 2).unwrap();

    let (serialized, len) = header.serialize();

    assert_eq!(len, header_len as usize);
    assert_eq!(&serialized[0..len], &buffer[0..header_len as usize]);
}
