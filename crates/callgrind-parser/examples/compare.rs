//! Compare semantic records across producer compression settings. Run on two
//! executions only with deterministic counters; do not compare timed events.
use callgrind_parser::{Location, Profile, Record, parse_reader};
use std::{fs::File, io::BufReader};

fn location(profile: &Profile, location: &Location) -> String {
    let resolve =
        |id: Option<callgrind_parser::StringId>| id.map(|id| profile.symbols.resolve(id).unwrap());
    let function = profile.symbols.function(location.function).unwrap();
    format!(
        "{:?}/{:?}/{:?}@{:?}:{:?}",
        resolve(function.object),
        resolve(function.file),
        resolve(function.name),
        resolve(location.file),
        location.positions
    )
}

fn canonical(profile: &Profile) -> Vec<String> {
    let mut result = Vec::new();
    for part in &profile.parts {
        let events: Vec<_> = part
            .header
            .events
            .iter()
            .map(|id| profile.symbols.resolve(*id).unwrap())
            .collect();
        result.push(format!("part {:?} {events:?}", part.header.positions));
        for item in &part.records {
            result.push(match &item.record {
                Record::Cost {
                    location: loc,
                    costs,
                } => format!("self {} {costs:?}", location(profile, loc)),
                Record::Call {
                    source,
                    target,
                    count,
                    costs,
                } => format!(
                    "call {} -> {} {count} {costs:?}",
                    location(profile, source),
                    location(profile, target)
                ),
                Record::Jump {
                    source,
                    target,
                    executed,
                    taken,
                } => format!(
                    "jump {} -> {} {executed} {taken:?}",
                    location(profile, source),
                    location(profile, target)
                ),
            });
        }
    }
    result
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths: Vec<_> = std::env::args_os().skip(1).collect();
    if paths.len() != 2 {
        return Err("usage: compare PROFILE_A PROFILE_B".into());
    }
    let a = canonical(&parse_reader(BufReader::new(File::open(&paths[0])?))?);
    let b = canonical(&parse_reader(BufReader::new(File::open(&paths[1])?))?);
    if a.len() != b.len() {
        return Err(format!("different record counts: {} and {}", a.len(), b.len()).into());
    }
    for (index, (a, b)) in a.iter().zip(&b).enumerate() {
        if a != b {
            return Err(format!("different semantic record {index}:\n{a}\n{b}").into());
        }
    }
    println!(
        "{} semantic records match (metadata and input line numbers excluded)",
        a.len()
    );
    Ok(())
}
