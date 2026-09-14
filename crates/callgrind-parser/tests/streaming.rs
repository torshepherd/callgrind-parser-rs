use callgrind_parser::*;
use std::io::{self, BufReader, Cursor, Read};

const INPUT: &str = "# callgrind format\nversion: 1\nevents: Ir Dr\nfl=naïve.cpp\nfn=(1) α\n1 3 2\ncfn=(1)\ncalls=4 1\n1 7\npart: 2\nevents: Ir\nfn=(1)\n2 9";

#[test]
fn every_small_buffer_size_matches_owned_parse_including_utf8_splits() {
    let expected = parse_profile(INPUT).unwrap();
    for size in 1..=INPUT.len() {
        let reader = BufReader::with_capacity(size, Cursor::new(INPUT.as_bytes()));
        assert_eq!(
            parse_reader(reader).unwrap(),
            expected,
            "buffer size {size}"
        );
    }
}

#[test]
fn event_order_headers_records_and_part_end() {
    let events = Decoder::new(Cursor::new(INPUT))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(events.len(), 7);
    assert!(matches!(events[0], ParseEvent::PartStart(_)));
    assert!(matches!(
        events[1],
        ParseEvent::Record(SpannedRecord {
            record: Record::Cost { .. },
            ..
        })
    ));
    assert!(matches!(
        events[2],
        ParseEvent::Record(SpannedRecord {
            record: Record::Call { .. },
            ..
        })
    ));
    assert!(matches!(events[3], ParseEvent::PartEnd { .. }));
    assert!(matches!(events[4], ParseEvent::PartStart(_)));
    assert!(matches!(events[5], ParseEvent::Record(_)));
    assert!(matches!(events[6], ParseEvent::PartEnd { .. }));
}

#[test]
fn stream_yields_before_reading_later_invalid_data_and_fuses_after_error() {
    let mut decoder = Decoder::new(Cursor::new("events: Ir\n1 2\n1 nope\n1 4\n"));
    assert!(matches!(
        decoder.next().unwrap().unwrap(),
        ParseEvent::PartStart(_)
    ));
    assert!(matches!(
        decoder.next().unwrap().unwrap(),
        ParseEvent::Record(_)
    ));
    assert_eq!(
        decoder.next().unwrap().unwrap_err().kind,
        ParseErrorKind::InvalidNumber
    );
    assert!(decoder.next().is_none());
    assert!(decoder.next().is_none());
}

#[test]
fn streaming_dictionary_does_not_grow_with_repeated_cost_rows() {
    let input = format!("events: Ir\nfn=main\n{}", "1 2\n".repeat(10_000));
    let mut decoder = Decoder::new(Cursor::new(input));
    let mut rows = 0;
    while let Some(event) = decoder.next() {
        if let ParseEvent::Record(record) = event.unwrap() {
            rows += 1;
            let Record::Cost { location, costs } = record.record else {
                panic!()
            };
            assert_eq!(&costs[..], [2]);
            let function = decoder.symbols().function(location.function).unwrap();
            assert_eq!(
                decoder.symbols().resolve(function.name.unwrap()),
                Some("main")
            );
        }
    }
    assert_eq!(rows, 10_000);
    assert_eq!(decoder.symbols().functions().len(), 1);
    assert_eq!(decoder.symbols().string_count(), 2);
    let dictionaries = decoder.into_symbols();
    let function = dictionaries.functions()[0];
    assert_eq!(dictionaries.resolve(function.name.unwrap()), Some("main"));
}

#[test]
fn invalid_utf8_is_reported_without_lossy_name_substitution() {
    let error = parse_reader(Cursor::new(b"events: Ir\nfn=\xff\n")).unwrap_err();
    assert_eq!((error.kind, error.line), (ParseErrorKind::InvalidUtf8, 2));
}

struct Broken;
impl Read for Broken {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("test read failure"))
    }
}

#[test]
fn io_errors_are_not_treated_as_successful_eof() {
    let error = parse_reader(BufReader::new(
        Cursor::new(b"events: Ir\n1 2\n").chain(Broken),
    ))
    .unwrap_err();
    assert_eq!((error.kind, error.line), (ParseErrorKind::Io, 3));
}

#[test]
fn oversized_line_returns_a_resource_error() {
    let input = format!("events: Ir\nfn={}\n", "x".repeat(8 * 1024 * 1024));
    let error = parse_profile(&input).unwrap_err();
    assert_eq!((error.kind, error.line), (ParseErrorKind::ResourceLimit, 2));
}

#[test]
fn owned_profile_outlives_input_and_can_be_shared_across_threads() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Profile>();
    let p = {
        let input = String::from(INPUT);
        parse_profile(&input).unwrap()
    };
    let Record::Cost { location, .. } = &p.parts[0].records[0].record else {
        panic!()
    };
    let name = p.symbols.function(location.function).unwrap().name.unwrap();
    assert_eq!(p.symbols.resolve(name), Some("α"));
}
