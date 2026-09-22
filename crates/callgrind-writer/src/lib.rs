//! A deliberately small streaming writer: one part, instr/line positions,
//! exclusive rows and call edges. No derived events, jumps or relative PCs.
//!
//! **Experimental — documentation status:** This crate is experimental. Its
//! documentation is fully LLM-generated and may contain errors or outdated claims.
//! The documentation will receive a review and cleanup pass before the 1.0 release.
use std::{
    collections::BTreeMap,
    io::{self, Write},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub name: String,
    pub description: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub object: String,
    pub file: String,
    pub name: String,
}
#[derive(Debug, Clone, Copy)]
pub struct Location<'a> {
    pub function: &'a Function,
    pub instruction: u64,
    pub line: u64,
}

/// Injective UTF-8 percent encoding of boundary spaces, other whitespace,
/// controls, '%' and leading
/// '(' (which could be mistaken for nested alias syntax). Empty has its own
/// reserved spelling. Generated names must be encoded exactly once.
pub fn escape_name(s: &str) -> String {
    if s.is_empty() {
        return "%EMPTY".into();
    }
    let mut out = String::new();
    for (i, c) in s.char_indices() {
        if (c.is_whitespace() && (c != ' ' || i == 0 || i + 1 == s.len()))
            || c.is_control()
            || c == '%'
            || (i == 0 && c == '(')
        {
            for b in c.to_string().as_bytes() {
                use std::fmt::Write;
                write!(out, "%{b:02X}").expect("writing String");
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn invalid(s: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, s)
}

pub struct Writer<W> {
    output: W,
    width: usize,
    totals: Vec<u64>,
    dictionaries: [BTreeMap<String, usize>; 3],
}
impl<W: Write> Writer<W> {
    pub fn new(mut output: W, events: &[Event], descriptions: &[String]) -> io::Result<Self> {
        if events.is_empty() {
            return Err(invalid("no events"));
        }
        let mut names = std::collections::BTreeSet::new();
        for event in events {
            if !event.name.starts_with(|c: char| c.is_ascii_alphabetic())
                || !event.name.chars().all(|c| c.is_ascii_alphanumeric())
                || !names.insert(&event.name)
            {
                return Err(invalid(
                    "event names must be unique ASCII alphanumeric identifiers",
                ));
            }
        }
        writeln!(
            output,
            "# callgrind format\nversion: 1\ncreator: callgrind-writer\npositions: instr line"
        )?;
        for description in descriptions {
            writeln!(output, "desc: Note: {}", escape_name(description))?;
        }
        for event in events {
            writeln!(
                output,
                "event: {} : {}",
                event.name,
                escape_name(&event.description)
            )?;
        }
        writeln!(
            output,
            "events: {}",
            events
                .iter()
                .map(|e| e.name.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        )?;
        Ok(Self {
            output,
            width: events.len(),
            totals: vec![0; events.len()],
            dictionaries: Default::default(),
        })
    }
    fn name(&mut self, tag: &str, namespace: usize, raw: &str) -> io::Result<()> {
        let names = &mut self.dictionaries[namespace];
        if let Some(id) = names.get(raw) {
            writeln!(self.output, "{tag}=({id})")
        } else {
            let id = names.len() + 1;
            names.insert(raw.into(), id);
            writeln!(self.output, "{tag}=({id}) {}", escape_name(raw))
        }
    }
    fn context(&mut self, f: &Function, callee: bool) -> io::Result<()> {
        let tags = if callee {
            ["cob", "cfl", "cfn"]
        } else {
            ["ob", "fl", "fn"]
        };
        for (ns, (tag, value)) in tags
            .into_iter()
            .zip([&f.object, &f.file, &f.name])
            .enumerate()
        {
            self.name(tag, ns, value)?;
        }
        Ok(())
    }
    fn check(&self, costs: &[u64]) -> io::Result<()> {
        if costs.len() != self.width {
            return Err(invalid("event vector width mismatch"));
        }
        Ok(())
    }
    fn row(&mut self, l: Location<'_>, costs: &[u64]) -> io::Result<()> {
        write!(self.output, "0x{:x} {}", l.instruction, l.line)?;
        for c in costs {
            write!(self.output, " {c}")?;
        }
        writeln!(self.output)
    }
    pub fn self_cost(&mut self, location: Location<'_>, costs: &[u64]) -> io::Result<()> {
        self.check(costs)?;
        let totals = self
            .totals
            .iter()
            .zip(costs)
            .map(|(a, b)| {
                a.checked_add(*b)
                    .ok_or_else(|| invalid("self totals overflow u64"))
            })
            .collect::<io::Result<Vec<_>>>()?;
        self.context(location.function, false)?;
        self.row(location, costs)?;
        self.totals = totals;
        Ok(())
    }
    pub fn call(
        &mut self,
        source: Location<'_>,
        target: Location<'_>,
        count: u64,
        costs: &[u64],
    ) -> io::Result<()> {
        self.check(costs)?;
        self.context(source.function, false)?;
        self.context(target.function, true)?;
        writeln!(
            self.output,
            "calls={count} 0x{:x} {}",
            target.instruction, target.line
        )?;
        self.row(source, costs)
    }
    pub fn finish(mut self) -> io::Result<W> {
        write!(self.output, "totals:")?;
        for c in &self.totals {
            write!(self.output, " {c}")?;
        }
        writeln!(self.output)?;
        self.output.flush()?;
        Ok(self.output)
    }
}
