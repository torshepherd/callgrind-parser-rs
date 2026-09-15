//! Test-only normalized export. All wire parsing belongs to callgrind-parser.
use callgrind_parser::{FunctionId, PositionKind, ProfileSymbols, Record, StringId, parse_reader};
use std::{collections::BTreeMap, error::Error, fs::File, io::BufReader};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Identity = [String; 3];
type Vector = Vec<u128>;

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn name(symbols: &ProfileSymbols, id: Option<StringId>) -> Result<String> {
    let value = id
        .map(|id| symbols.resolve(id).ok_or("invalid string ID"))
        .transpose()?;
    Ok(hex(match value {
        None | Some("???") => "",
        Some(s) => s,
    }))
}

fn identity(symbols: &ProfileSymbols, id: FunctionId) -> Result<Identity> {
    let f = symbols.function(id).ok_or("invalid function ID")?;
    Ok([
        name(symbols, f.object)?,
        name(symbols, f.file)?,
        name(symbols, f.name)?,
    ])
}

fn add(sum: &mut [u128], costs: &[u64]) -> Result<()> {
    if sum.len() != costs.len() {
        return Err("event vector width mismatch".into());
    }
    for (s, c) in sum.iter_mut().zip(costs) {
        *s = s.checked_add(u128::from(*c)).ok_or("counter overflow")?;
    }
    Ok(())
}

fn vector(values: &[u128]) -> String {
    values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        return Err("usage: reference_export PROFILE".into());
    }
    let profile = parse_reader(BufReader::new(File::open(&args[0])?))?;
    if profile.parts.is_empty() {
        return Err("no parts".into());
    }
    for (p, part) in profile.parts.iter().enumerate() {
        let width = part.header.events.len();
        let events = part
            .header
            .events
            .iter()
            .map(|id| {
                profile
                    .symbols
                    .resolve(*id)
                    .map(hex)
                    .ok_or("invalid event ID")
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut totals = vec![0; width];
        let mut functions: BTreeMap<Identity, Vector> = BTreeMap::new();
        let mut edges: BTreeMap<(Identity, Identity), (u128, Vector)> = BTreeMap::new();
        let mut lines: BTreeMap<(Identity, String, u64), Vector> = BTreeMap::new();
        for row in &part.records {
            match &row.record {
                Record::Cost { location, costs } => {
                    let key = identity(&profile.symbols, location.function)?;
                    add(&mut totals, costs)?;
                    add(
                        functions
                            .entry(key.clone())
                            .or_insert_with(|| vec![0; width]),
                        costs,
                    )?;
                    if let Some(line) = part
                        .header
                        .position(&location.positions, PositionKind::Line)
                        .filter(|n| *n != 0)
                    {
                        let file = name(&profile.symbols, location.file)?;
                        add(
                            lines
                                .entry((key, file, line))
                                .or_insert_with(|| vec![0; width]),
                            costs,
                        )?;
                    }
                }
                Record::Call {
                    source,
                    target,
                    count,
                    costs,
                } => {
                    let key = (
                        identity(&profile.symbols, source.function)?,
                        identity(&profile.symbols, target.function)?,
                    );
                    let entry = edges.entry(key).or_insert_with(|| (0, vec![0; width]));
                    entry.0 = entry
                        .0
                        .checked_add(u128::from(*count))
                        .ok_or("call count overflow")?;
                    add(&mut entry.1, costs)?;
                }
                Record::Jump { .. } => {}
            }
        }
        if let Some(expected) = &part.totals
            && expected.iter().copied().map(u128::from).collect::<Vec<_>>() != totals
        {
            return Err(format!("part {p}: declared totals differ from self costs").into());
        }
        println!("P\t{p}\t{}", events.join(","));
        println!("T\t{p}\t{}", vector(&totals));
        for (key, costs) in functions {
            println!("F\t{p}\t{}\t{}", key.join("\t"), vector(&costs));
        }
        for ((caller, callee), (count, costs)) in edges {
            println!(
                "E\t{p}\t{}\t{}\t{count}\t{}",
                caller.join("\t"),
                callee.join("\t"),
                vector(&costs)
            );
        }
        for ((key, file, line), costs) in lines {
            println!(
                "L\t{p}\t{}\t{file}\t{line}\t{}",
                key.join("\t"),
                vector(&costs)
            );
        }
    }
    Ok(())
}
