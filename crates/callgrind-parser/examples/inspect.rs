//! Optional real-profile validation; ordinary Rust tests launch no subprocesses.
use callgrind_parser::{Record, parse_reader};
use std::{fs::File, io::BufReader};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths: Vec<_> = std::env::args_os().skip(1).collect();
    if paths.is_empty() {
        return Err("usage: inspect PROFILE [PROFILE ...]".into());
    }
    for path in paths {
        let profile = parse_reader(BufReader::new(File::open(&path)?))?;
        for (index, part) in profile.parts.iter().enumerate() {
            let mut self_costs = vec![0u128; part.header.events.len()];
            let (mut calls, mut jumps) = (0usize, 0usize);
            for record in &part.records {
                match &record.record {
                    Record::Cost { costs, .. } => {
                        for (sum, value) in self_costs.iter_mut().zip(costs) {
                            *sum = sum.checked_add(u128::from(*value)).ok_or("sum overflow")?;
                        }
                    }
                    Record::Call { .. } => calls += 1,
                    Record::Jump { .. } => jumps += 1,
                }
            }
            if let Some(totals) = &part.totals {
                let expected: Vec<_> = totals.iter().copied().map(u128::from).collect();
                if expected != self_costs {
                    return Err(format!("{path:?}, part {index}: self costs {self_costs:?} differ from totals {expected:?}").into());
                }
            }
            println!(
                "{path:?}\tpart={index}\tfunctions={}\trecords={}\tcalls={calls}\tjumps={jumps}\tself={self_costs:?}\ttotals_checked={}",
                profile.symbols.functions().len(),
                part.records.len(),
                part.totals.is_some()
            );
        }
    }
    Ok(())
}
