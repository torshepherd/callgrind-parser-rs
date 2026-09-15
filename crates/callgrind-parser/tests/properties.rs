use callgrind_parser::*;
use proptest::{
    prelude::*,
    test_runner::{Config, RngSeed},
};
use std::io::{BufReader, Cursor};

proptest! {
    #![proptest_config(Config {
        cases: 128, rng_seed: RngSeed::Fixed(0xCA11_6A1D),
        failure_persistence: None, ..Config::default()
    })]

    #[test]
    fn decimal_and_hex_preserve_all_u64_values(position in any::<u64>(), value in any::<u64>()) {
        let decimal = format!("events: Ir\n{position} {value}\n");
        let hex = format!("events: Ir\n0x{position:x} 0x{value:x}\n");
        let p = parse_profile(&decimal).unwrap();
        prop_assert_eq!(&p, &parse_profile(&hex).unwrap());
        let Record::Cost { location, costs } = &p.parts[0].records[0].record else { panic!() };
        prop_assert_eq!(&location.positions[..],[position]);
        prop_assert_eq!(&costs[..],[value]);
    }

    #[test]
    fn compressed_and_absolute_positions_have_identical_semantics(values in prop::collection::vec(0u64..1_000_000, 1..80)) {
        let mut absolute = String::from("events: Ir\n");
        let mut compressed = absolute.clone();
        let mut previous = 0;
        for &value in &values {
            absolute.push_str(&format!("{value} 1\n"));
            let token = if value == previous { "*".into() }
                else if value > previous { format!("+{}",value-previous) }
                else { format!("-{}",previous-value) };
            compressed.push_str(&format!("{token} 1\n"));
            previous = value;
        }
        let p = parse_profile(&compressed).unwrap();
        prop_assert_eq!(&p,&parse_profile(&absolute).unwrap());
        let decoded: Vec<_> = p.parts[0].records.iter().map(|record| match &record.record {
            Record::Cost { location, .. } => location.positions[0], _ => panic!(),
        }).collect();
        prop_assert_eq!(decoded,values);
    }

    #[test]
    fn trailing_zero_omission_preserves_dense_costs(values in prop::collection::vec(any::<u64>(), 0..12)) {
        let events = (0..12).map(|i|format!("E{i}")).collect::<Vec<_>>().join(" ");
        let costs = values.iter().map(ToString::to_string).collect::<Vec<_>>().join(" ");
        let p = parse_profile(&format!("events: {events}\n1 {costs}\n")).unwrap();
        let Record::Cost { costs, .. } = &p.parts[0].records[0].record else { panic!() };
        let mut expected = values;
        expected.resize(12,0);
        prop_assert_eq!(&costs[..],expected);
    }

    #[test]
    fn arbitrary_bytes_never_panic_and_chunking_does_not_change_results(bytes in prop::collection::vec(any::<u8>(),0..512), size in 1usize..32) {
        let direct = parse_reader(Cursor::new(&bytes));
        let buffered = parse_reader(BufReader::with_capacity(size,Cursor::new(&bytes)));
        prop_assert_eq!(direct,buffered);
    }

    #[test]
    fn arbitrary_utf8_body_is_either_valid_or_a_located_error(body in ".{0,256}") {
        let input = format!("events: Ir\n{body}\n");
        if let Err(error) = parse_profile(&input) {
            prop_assert!(error.line >= 1);
            prop_assert!(error.line <= input.lines().count().max(1));
        }
    }
}
