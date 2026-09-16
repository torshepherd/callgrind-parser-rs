use callgrind_parser::{Record, parse_profile};
use callgrind_writer::*;

fn events() -> Vec<Event> {
    vec![Event {
        name: "E0".into(),
        description: "test".into(),
    }]
}
fn function(name: &str) -> Function {
    Function {
        object: "object".into(),
        file: "file".into(),
        name: name.into(),
    }
}

#[test]
fn escaping_is_injective_and_prevents_record_injection() {
    let inputs = [
        "",
        "%EMPTY",
        "(1)",
        "%281)",
        " a ",
        "a b",
        "a\nb",
        "a\tb",
        "a%0Ab",
        "é",
        "a\rfn=evil",
    ];
    let encoded: std::collections::BTreeSet<_> = inputs.iter().map(|s| escape_name(s)).collect();
    assert_eq!(encoded.len(), inputs.len());
    for raw in inputs {
        let f = function(raw);
        let mut w = Writer::new(Vec::new(), &events(), &[]).unwrap();
        w.self_cost(
            Location {
                function: &f,
                instruction: 0,
                line: 0,
            },
            &[7],
        )
        .unwrap();
        let p = parse_profile(&String::from_utf8(w.finish().unwrap()).unwrap()).unwrap();
        assert_eq!(p.parts[0].records.len(), 1);
        let f = &p.symbols.functions()[0];
        assert_eq!(
            p.symbols.resolve(f.name.unwrap()),
            Some(escape_name(raw).as_str())
        );
    }
}

#[test]
fn absolute_targets_aliases_and_self_rows_after_calls() {
    let a = function("same");
    let mut b = a.clone();
    b.object = "other".into();
    let a = Location {
        function: &a,
        instruction: 0x100,
        line: 2,
    };
    let b = Location {
        function: &b,
        instruction: 0x9876,
        line: 88,
    };
    let mut w = Writer::new(Vec::new(), &events(), &[]).unwrap();
    w.self_cost(a, &[3]).unwrap();
    w.call(a, b, 0, &[9]).unwrap();
    w.call(b, a, 5, &[2]).unwrap();
    w.self_cost(a, &[4]).unwrap();
    w.self_cost(b, &[9]).unwrap();
    let text = String::from_utf8(w.finish().unwrap()).unwrap();
    let p = parse_profile(&text).unwrap();
    assert_eq!(p.parts[0].totals.as_deref(), Some(&[16][..]));
    let Record::Call {
        source,
        target,
        count,
        costs,
    } = &p.parts[0].records[1].record
    else {
        panic!("missing call")
    };
    assert_eq!(*count, 0);
    assert_eq!(&**costs, [9]);
    assert_eq!(&target.positions[..], [0x9876, 88]);
    assert_ne!(source.function, target.function);
    assert!(matches!(p.parts[0].records[3].record, Record::Cost { .. }));
}

#[test]
fn invalid_events_width_and_overflow_are_errors() {
    assert!(Writer::new(Vec::new(), &[], &[]).is_err());
    assert!(Writer::new(Vec::new(), &[events()[0].clone(), events()[0].clone()], &[]).is_err());
    assert!(
        Writer::new(
            Vec::new(),
            &[Event {
                name: "bad\nevents:".into(),
                description: "".into()
            }],
            &[]
        )
        .is_err()
    );
    let f = function("f");
    let l = Location {
        function: &f,
        instruction: 0,
        line: 0,
    };
    let mut w = Writer::new(Vec::new(), &events(), &[]).unwrap();
    assert!(w.self_cost(l, &[]).is_err());
    assert!(w.call(l, l, 0, &[]).is_err());
    w.self_cost(l, &[u64::MAX]).unwrap();
    assert!(w.self_cost(l, &[1]).is_err());
    let p = parse_profile(&String::from_utf8(w.finish().unwrap()).unwrap()).unwrap();
    assert_eq!(p.parts[0].records.len(), 1);
}

#[test]
fn flush_failure_propagates() {
    struct FailFlush;
    impl std::io::Write for FailFlush {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Err(std::io::Error::other("flush"))
        }
    }
    assert!(
        Writer::new(FailFlush, &events(), &[])
            .unwrap()
            .finish()
            .is_err()
    );
}
