use pprof_profile::{proto::*, *};
use prost::Message;

fn fixture() -> Profile {
    Profile {
        string_table: vec!["".into(), "cpu".into(), "nanoseconds".into()],
        sample_type: vec![ValueType { r#type: 1, unit: 2 }],
        mapping: vec![Mapping {
            id: 7,
            filename: 1,
            build_id: 2,
            memory_start: 4096,
            memory_limit: 8192,
            file_offset: 99,
            has_functions: true,
            has_filenames: true,
            has_line_numbers: true,
            has_inline_frames: true,
        }],
        function: vec![Function {
            id: 8,
            name: 1,
            system_name: 2,
            filename: 1,
            start_line: 42,
        }],
        location: vec![Location {
            id: 9,
            mapping_id: 7,
            address: 4100,
            line: vec![Line {
                function_id: 8,
                line: 43,
                column: 3,
            }],
            is_folded: true,
        }],
        sample: vec![Sample {
            location_id: vec![9],
            value: vec![-9007199254740993],
            label: vec![
                Label {
                    key: 1,
                    str: 2,
                    ..Default::default()
                },
                Label {
                    key: 1,
                    num: -99,
                    num_unit: 2,
                    ..Default::default()
                },
            ],
        }],
        drop_frames: 1,
        keep_frames: 2,
        time_nanos: 123,
        duration_nanos: 456,
        period_type: Some(ValueType { r#type: 1, unit: 2 }),
        period: 1000,
        comment: vec![1, 2],
        default_sample_type: 1,
        doc_url: 2,
    }
}

#[test]
fn all_fields_raw_and_gzip_roundtrip() {
    let p = fixture();
    assert_eq!(read(p.encode_to_vec().as_slice(), 4096).unwrap(), p);
    let mut a = Vec::new();
    write(&p, &mut a).unwrap();
    let mut b = Vec::new();
    write(&p, &mut b).unwrap();
    assert_eq!(a, b);
    assert_eq!(read(a.as_slice(), 4096).unwrap(), p);
}

#[test]
fn invalid_ids_references_strings_and_widths() {
    let mutations: Vec<fn(&mut Profile)> = vec![
        |p| p.string_table[0] = "bad".into(),
        |p| p.sample_type.clear(),
        |p| p.sample_type[0].unit = -1,
        |p| p.doc_url = 99,
        |p| p.mapping[0].id = 0,
        |p| p.mapping.push(p.mapping[0].clone()),
        |p| p.function.push(p.function[0].clone()),
        |p| p.location.push(p.location[0].clone()),
        |p| p.location[0].mapping_id = 99,
        |p| p.location[0].line[0].function_id = 99,
        |p| p.sample[0].location_id = vec![99],
        |p| p.sample[0].value.clear(),
        |p| p.sample[0].label[0].num = 1,
        |p| p.sample[0].label[0].key = 99,
    ];
    for mutate in mutations {
        let mut p = fixture();
        mutate(&mut p);
        assert!(validate(&p).is_err());
    }
}

#[test]
fn gzip_integrity_limits_and_trailing_data() {
    let mut bytes = Vec::new();
    write(&fixture(), &mut bytes).unwrap();
    assert!(read(bytes.as_slice(), 1).is_err());
    assert!(read(bytes.as_slice(), 0).is_err());
    assert!(read(bytes.as_slice(), u64::MAX).is_err());
    assert!(read(&bytes[..bytes.len() - 1], 4096).is_err());
    let mut corrupt = bytes.clone();
    let n = corrupt.len();
    corrupt[n - 8] ^= 1;
    assert!(read(corrupt.as_slice(), 4096).is_err());
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(read(trailing.as_slice(), 4096).is_err());
    let mut concatenated = bytes.clone();
    concatenated.extend(&bytes);
    assert!(read(concatenated.as_slice(), 4096).is_err());
    let mut p = fixture();
    p.string_table.push("x".repeat(10000));
    bytes.clear();
    write(&p, &mut bytes).unwrap();
    assert!(bytes.len() < 1000);
    assert!(read(bytes.as_slice(), 1000).is_err());
}

#[test]
fn unknown_fields_are_accepted_but_not_preserved() {
    let p = fixture();
    let mut raw = p.encode_to_vec();
    raw.extend([0xa0, 0x06, 0x01]); // unknown field 100, varint 1
    assert_eq!(read(raw.as_slice(), 4096).unwrap(), p);
    assert!(read(&[0xff][..], 4096).is_err());
}

#[test]
fn output_failures_propagate() {
    struct Fail;
    impl std::io::Write for Fail {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("test"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    assert!(write(&fixture(), Fail).is_err());
}
