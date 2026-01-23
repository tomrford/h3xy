use super::*;

#[test]
fn test_output_record_type_requires_length() {
    let mut args = Args::default();
    let result = parse_option(&mut args, "XS::2");
    assert!(result.is_err());
}

#[test]
fn test_parse_ad_no_separator_hex() {
    let mut args = Args::default();
    parse_option(&mut args, "AD10").unwrap();
    assert_eq!(args.align_address, Some(0x10));
}

#[test]
fn test_parse_af_no_separator_hex() {
    let mut args = Args::default();
    parse_option(&mut args, "AF0A").unwrap();
    assert_eq!(args.align_fill, 0x0A);
}
