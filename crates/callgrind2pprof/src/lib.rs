//! Exact exclusive-cost export. Aggregate edges do not establish sampled stacks.
use callgrind_parser::{PositionKind, Profile, ProfileSymbols, Record, StringId};
use pprof_profile::{Error, proto};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
};

#[derive(Debug, Clone)]
pub struct Options {
    /// Zero-based part index; required if the input contains multiple parts.
    pub part: Option<usize>,
    /// Event-name to unit overrides; values are not scaled.
    pub units: BTreeMap<String, String>,
    pub max_locations: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            part: None,
            units: BTreeMap::new(),
            max_locations: 1_000_000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Report {
    profile: proto::Profile,
    totals: Vec<u64>,
    warnings: Vec<String>,
    part: usize,
}
impl Report {
    pub fn profile(&self) -> &proto::Profile {
        &self.profile
    }
    pub fn totals(&self) -> &[u64] {
        &self.totals
    }
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
    pub fn part(&self) -> usize {
        self.part
    }
    pub fn write(&self, output: impl Write) -> Result<(), Error> {
        pprof_profile::write(&self.profile, output)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FunctionKey {
    object: Option<String>,
    defining_file: Option<String>,
    name: Option<String>,
    source_file: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LocationKey {
    function: FunctionKey,
    instruction: Option<u64>,
    block: Option<u64>,
    line: Option<u64>,
}

fn resolve(symbols: &ProfileSymbols, id: Option<StringId>) -> Result<Option<String>, Error> {
    id.map(|id| {
        symbols
            .resolve(id)
            .map(String::from)
            .ok_or_else(|| Error("invalid string ID".into()))
    })
    .transpose()
}
fn intern(p: &mut proto::Profile, strings: &mut BTreeMap<String, i64>, value: &str) -> i64 {
    if let Some(id) = strings.get(value) {
        return *id;
    }
    let id = p.string_table.len() as i64;
    p.string_table.push(value.into());
    strings.insert(value.into(), id);
    id
}
fn label(
    p: &mut proto::Profile,
    strings: &mut BTreeMap<String, i64>,
    labels: &mut Vec<proto::Label>,
    key: &str,
    value: String,
) {
    labels.push(proto::Label {
        key: intern(p, strings, key),
        str: intern(p, strings, &value),
        ..Default::default()
    });
}
fn sum(into: &mut [u64], values: &[u64]) -> Result<(), Error> {
    for (a, b) in into.iter_mut().zip(values) {
        *a = a
            .checked_add(*b)
            .filter(|n| *n <= i64::MAX as u64)
            .ok_or_else(|| Error("exclusive event sum exceeds pprof's signed i64 range".into()))?;
    }
    Ok(())
}
/// Conservative units for standard event counters; no time-unit inference.
pub fn default_unit(event: &str) -> &'static str {
    match event {
        "Ir" | "Dr" | "Dw" | "I1mr" | "D1mr" | "D1mw" | "ILmr" | "DLmr" | "DLmw" | "I2mr"
        | "D2mr" | "D2mw" | "Bc" | "Bcm" | "Bi" | "Bim" | "sysCount" => "count",
        _ => "callgrind_raw",
    }
}

pub fn convert(input: &Profile, options: &Options) -> Result<Report, Error> {
    if options.max_locations == 0 {
        return Err(Error("max-locations must be positive".into()));
    }
    let index = match (options.part, input.parts.len()) {
        (Some(i), _) if i < input.parts.len() => i,
        (Some(_), _) => return Err(Error("part index is out of range".into())),
        (None, 1) => 0,
        (None, 0) => return Err(Error("input has no parts".into())),
        (None, _) => return Err(Error("multipart input requires --part (zero-based)".into())),
    };
    let part = &input.parts[index];
    let header = &part.header;
    let events = header
        .events
        .iter()
        .map(|id| resolve(&input.symbols, Some(*id)).map(|s| s.expect("Some ID")))
        .collect::<Result<Vec<_>, _>>()?;
    let width = events.len();
    if width == 0
        || events.iter().any(String::is_empty)
        || events.iter().collect::<BTreeSet<_>>().len() != width
    {
        return Err(Error("events must be nonempty and unique".into()));
    }
    if header.positions.is_empty()
        || header.positions.len() > 3
        || header.positions.iter().collect::<BTreeSet<_>>().len() != header.positions.len()
    {
        return Err(Error("invalid position layout".into()));
    }
    for (event, unit) in &options.units {
        if !events.contains(event) || unit.is_empty() || unit.chars().any(char::is_control) {
            return Err(Error(format!("invalid unit override for {event}")));
        }
    }
    let mut rows: BTreeMap<LocationKey, Vec<u64>> = BTreeMap::new();
    let mut totals = vec![0; width];
    for row in &part.records {
        let Record::Cost { location, costs } = &row.record else {
            continue;
        };
        if costs.len() != width || location.positions.len() != header.positions.len() {
            return Err(Error(format!(
                "line {}: inconsistent vector width",
                row.line
            )));
        }
        if costs.iter().all(|v| *v == 0) {
            continue;
        }
        let f = input
            .symbols
            .function(location.function)
            .ok_or_else(|| Error("invalid function ID".into()))?;
        let key = LocationKey {
            function: FunctionKey {
                object: resolve(&input.symbols, f.object)?,
                defining_file: resolve(&input.symbols, f.file)?,
                name: resolve(&input.symbols, f.name)?,
                source_file: resolve(&input.symbols, location.file)?,
            },
            instruction: header.position(&location.positions, PositionKind::Instruction),
            block: header.position(&location.positions, PositionKind::BasicBlock),
            line: header.position(&location.positions, PositionKind::Line),
        };
        if key.line.is_some_and(|n| n > i64::MAX as u64) {
            return Err(Error("source line exceeds pprof's signed i64 range".into()));
        }
        if !rows.contains_key(&key) && rows.len() >= options.max_locations {
            return Err(Error("location count exceeds max-locations".into()));
        }
        sum(&mut totals, costs)?;
        sum(rows.entry(key).or_insert_with(|| vec![0; width]), costs)?;
    }
    if part
        .totals
        .as_deref()
        .is_some_and(|declared| declared != totals)
    {
        return Err(Error(
            "declared totals differ from summed exclusive costs".into(),
        ));
    }
    let mut profile = proto::Profile {
        string_table: vec![String::new()],
        ..Default::default()
    };
    let mut strings = BTreeMap::from([(String::new(), 0)]);
    let mut warnings = vec!["Flat exclusive-cost export: call edges, counts and jumps are omitted; caller stacks are not reconstructed.".into(),
        "Original object/position data are labels; no runtime PC, mapping, build ID, inline chain or timing is invented.".into()];
    for event in &events {
        let unit = options
            .units
            .get(event)
            .map(String::as_str)
            .unwrap_or_else(|| default_unit(event));
        if unit == "callgrind_raw" {
            warnings.push(format!("{event}: unknown or producer-dependent units retained as callgrind_raw; use --unit {event}=UNIT to specify without scaling."));
        }
        let t = intern(&mut profile, &mut strings, event);
        let u = intern(&mut profile, &mut strings, unit);
        profile
            .sample_type
            .push(proto::ValueType { r#type: t, unit: u });
    }
    profile.default_sample_type = profile.sample_type[0].r#type;
    let mut comments = warnings.clone();
    comments.push(format!("callgrind2pprof: part index={index}; totals are self costs; labels with v: prefix preserve original strings, including empty values"));
    for (name, value) in [
        ("pid", header.metadata.pid),
        ("thread", header.metadata.thread),
        ("part", header.metadata.part),
    ] {
        if let Some(value) = value {
            comments.push(format!("callgrind {name}={value}"));
        }
    }
    if let Some(command) = &header.metadata.command {
        comments.push(format!("callgrind cmd={command}"));
    }
    if let Some(summary) = &header.summary {
        comments.push(format!(
            "callgrind summary={summary:?}; not used as sample totals"
        ));
    }
    for definition in &header.event_definitions {
        if let Some(name) = resolve(&input.symbols, Some(definition.name))? {
            comments.push(format!(
                "callgrind event {name}: {}{}",
                definition.long_name.as_deref().unwrap_or(""),
                if definition.terms.is_some() {
                    " (derived expression not evaluated)"
                } else {
                    ""
                }
            ));
        }
    }
    for comment in comments {
        let id = intern(&mut profile, &mut strings, &comment);
        profile.comment.push(id);
    }
    let mut functions: BTreeMap<FunctionKey, u64> = BTreeMap::new();
    for (key, costs) in rows {
        let function_id = if let Some(id) = functions.get(&key.function) {
            *id
        } else {
            let id = functions.len() as u64 + 1;
            let raw_name = key
                .function
                .name
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or("<unknown>");
            let name = intern(
                &mut profile,
                &mut strings,
                &format!("{raw_name} [callgrind:f{id}]"),
            );
            let filename = intern(
                &mut profile,
                &mut strings,
                key.function.source_file.as_deref().unwrap_or(""),
            );
            profile.function.push(proto::Function {
                id,
                name,
                filename,
                ..Default::default()
            });
            functions.insert(key.function.clone(), id);
            id
        };
        let id = profile.location.len() as u64 + 1;
        profile.location.push(proto::Location {
            id,
            line: vec![proto::Line {
                function_id,
                line: key.line.unwrap_or(0) as i64,
                column: 0,
            }],
            ..Default::default()
        });
        let mut labels = Vec::new();
        for (name, value) in [
            ("object", &key.function.object),
            ("function", &key.function.name),
            ("defining_file", &key.function.defining_file),
            ("source_file", &key.function.source_file),
        ] {
            if let Some(value) = value {
                label(
                    &mut profile,
                    &mut strings,
                    &mut labels,
                    &format!("callgrind.{name}"),
                    format!("v:{value}"),
                );
            }
        }
        for (name, value) in [
            ("instruction", key.instruction),
            ("basic_block", key.block),
            ("line", key.line),
        ] {
            if let Some(value) = value {
                label(
                    &mut profile,
                    &mut strings,
                    &mut labels,
                    &format!("callgrind.{name}"),
                    value.to_string(),
                );
            }
        }
        profile.sample.push(proto::Sample {
            location_id: vec![id],
            value: costs.into_iter().map(|n| n as i64).collect(),
            label: labels,
        });
    }
    pprof_profile::validate(&profile)?;
    Ok(Report {
        profile,
        totals,
        warnings,
        part: index,
    })
}
