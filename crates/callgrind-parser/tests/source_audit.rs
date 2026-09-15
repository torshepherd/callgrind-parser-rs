//! Minimal contracts from the pinned source audit in docs/SOURCE-AUDIT.md.
use callgrind_parser::*;

#[test]
fn combined_dumps_inherit_process_metadata_but_reset_part_state() {
    let p = parse_profile(
        "pid: 42\ncmd: ./program\npart: 1\nthread: 7\n\
         desc: Trigger: first\npositions: instr line\nevents: Ir Dr\n\
         summary: 2 1\nfl=(1) main.c\nfn=(1) main\n0x10 1 2 1\ntotals: 2 1\n\
         part: 2\nevents: Dr Ir\nfl=(1)\nfn=(1)\n1 3 4\ntotals: 3 4\n\
         pid: 99\ncmd: ./other\npart: 3\nevents: Ir\n1 5\n",
    )
    .unwrap();
    assert_eq!(p.parts.len(), 3);
    let h = &p.parts[1].header;
    assert_eq!(h.metadata.pid, Some(42));
    assert_eq!(h.metadata.command.as_deref(), Some("./program"));
    assert_eq!(h.metadata.part, Some(2));
    assert_eq!(h.metadata.thread, None);
    assert!(h.metadata.descriptions.is_empty());
    assert_eq!(h.positions, [PositionKind::Line]);
    assert_eq!(h.summary, None);
    assert_eq!(p.symbols.resolve(h.events[0]), Some("Dr"));
    assert_eq!(p.parts[2].header.metadata.pid, Some(99));
    assert_eq!(
        p.parts[2].header.metadata.command.as_deref(),
        Some("./other")
    );
}

#[test]
fn active_call_after_dump_has_zero_count_and_nonzero_inclusive_cost() {
    let p = parse_profile(include_str!("fixtures/zero-call-count.callgrind")).unwrap();
    let Record::Call {
        count,
        costs,
        target,
        ..
    } = &p.parts[0].records[1].record
    else {
        panic!("zero call count must not turn inclusive cost into self cost");
    };
    assert_eq!(*count, 0);
    assert_eq!(&costs[..], [7]);
    assert_eq!(&target.positions[..], [10]);
    let self_sum: u64 = p.parts[0]
        .records
        .iter()
        .filter_map(|r| match &r.record {
            Record::Cost { costs, .. } => Some(costs[0]),
            _ => None,
        })
        .sum();
    assert_eq!(self_sum, 9);
}

#[test]
fn recursive_edge_cost_is_retained_separately_from_self_cost() {
    let p = parse_profile(include_str!("fixtures/recursive-cost.callgrind")).unwrap();
    let Record::Call {
        source,
        target,
        count,
        costs,
    } = &p.parts[0].records[3].record
    else {
        panic!("recursive call");
    };
    assert_eq!(source.function, target.function);
    assert_eq!(*count, 2);
    assert_eq!(&costs[..], [5]);
    let Record::Cost { costs, .. } = &p.parts[0].records[2].record else {
        panic!("recursive function self cost");
    };
    assert_eq!(&costs[..], [7]);
}

#[test]
fn function_identity_keeps_full_paths_and_tuple_boundaries() {
    let p = parse_profile(
        "events: Ir\nob=/one/lib.so\nfl=/one/main.c\nfn=f\n1 1\n\
         ob=/two/lib.so\nfl=/two/main.c\nfn=f\n1 2\n\
         ob=c\nfl=b\nfn=a\n1 3\nob=c\nfl=ab\nfn=\n1 4\n",
    )
    .unwrap();
    let ids: std::collections::HashSet<_> = p.parts[0]
        .records
        .iter()
        .map(|r| {
            let Record::Cost { location, .. } = &r.record else {
                panic!("self cost")
            };
            location.function
        })
        .collect();
    assert_eq!(ids.len(), 4);
}

#[test]
fn producer_extensions_fail_explicitly_without_losing_costs() {
    for (body, kind) in [
        (
            "fn=(1) main\nfn=(2) (1)'2\n",
            ParseErrorKind::UnsupportedExtension,
        ),
        ("bb=0x10 2 1\n", ParseErrorKind::UnknownBodyLine),
        ("rec=1\n", ParseErrorKind::UnknownBodyLine),
        ("frfn=caller\n", ParseErrorKind::UnknownBodyLine),
        (
            "fn=main\nrcalls=1 2\n1 3\n",
            ParseErrorKind::UnknownBodyLine,
        ),
        ("1:3 4\n", ParseErrorKind::InvalidNumber),
    ] {
        let error = parse_profile(&format!("events: Ir\n{body}")).unwrap_err();
        assert_eq!(error.kind, kind, "{body}");
    }
}
