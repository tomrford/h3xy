use super::*;
use super::super::types::ChecksumTarget;

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

#[test]
fn test_parse_dp_signature_subset_option() {
    let mut args = Args::default();
    parse_option(&mut args, "DP32:@append:key.pem;sig.bin").unwrap();
    let dp = args.data_processing.expect("data processing parsed");
    assert_eq!(dp.method, 32);
    assert!(matches!(dp.placement, Some(ChecksumTarget::Append)));
    assert_eq!(dp.key_info, "key.pem");
}

#[test]
fn test_parse_sv_option() {
    let mut args = Args::default();
    parse_option(&mut args, "SV4:pub.pem!sig.bin").unwrap();
    let sv = args.signature_verify.expect("signature verification parsed");
    assert_eq!(sv.method, 4);
    assert_eq!(sv.key_info, "pub.pem");
    assert_eq!(sv.signature_info, "sig.bin");
}
