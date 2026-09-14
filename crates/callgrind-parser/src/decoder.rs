use crate::{lex, *};
use std::{
    collections::{HashMap, VecDeque},
    io::{BufRead, Read},
};

enum Association {
    Call {
        target: Location,
        count: u64,
    },
    Jump {
        target: Location,
        executed: u64,
        taken: Option<u64>,
    },
}

/// Incrementally decode BufRead input, including buffers splitting UTF-8.
/// Only one line, the current header, dictionaries, and a small output queue
/// are retained. Emitted records are not retained. Nom complete parsers run
/// after line framing. After an error this iterator is fused.
pub struct Decoder<R> {
    reader: R,
    line: usize,
    buffer: String,
    done: bool,
    queue: VecDeque<ParseEvent>,
    header: FileHeader,
    version_seen: bool,
    symbols: Symbols,
    names: [HashMap<u64, StringId>; 3],
    part: PartHeader,
    part_line: Option<usize>,
    summary_line: usize,
    started: bool,
    totals: Option<Costs>,
    previous: Positions,
    object: Option<StringId>,
    file: Option<StringId>,
    definition_file: Option<StringId>,
    function_name: Option<StringId>,
    function: Option<FunctionId>,
    called_object: Option<StringId>,
    called_file: Option<StringId>,
    called_function: Option<FunctionId>,
    jump_file: Option<StringId>,
    jump_function: Option<FunctionId>,
    pending: Option<(usize, Association)>,
}

impl<R: BufRead> Decoder<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line: 0,
            buffer: String::new(),
            done: false,
            queue: VecDeque::new(),
            header: FileHeader::default(),
            version_seen: false,
            symbols: Symbols::default(),
            names: Default::default(),
            part: PartHeader::default(),
            part_line: None,
            summary_line: 0,
            started: false,
            totals: None,
            previous: Positions::new(),
            object: None,
            file: None,
            definition_file: None,
            function_name: None,
            function: None,
            called_object: None,
            called_file: None,
            called_function: None,
            jump_file: None,
            jump_function: None,
            pending: None,
        }
    }

    pub fn header(&self) -> &FileHeader {
        &self.header
    }
    pub fn symbols(&self) -> &Symbols {
        &self.symbols
    }

    /// Retain dictionaries after consuming a stream. Calling this before EOF
    /// cancels decoding; it does not validate the unread remainder.
    pub fn into_symbols(self) -> ProfileSymbols {
        self.symbols.freeze()
    }

    fn error(&self, kind: ParseErrorKind) -> ParseError {
        ParseError {
            kind,
            line: self.line.max(1),
        }
    }

    fn intern(&mut self, text: &str) -> Result<StringId, ParseError> {
        self.symbols
            .strings
            .try_get_or_intern(text)
            .map(StringId)
            .map_err(|_| self.error(ParseErrorKind::ResourceLimit))
    }

    fn function_id(&mut self, value: Function) -> Result<FunctionId, ParseError> {
        if let Some(id) = self.symbols.function_ids.get(&value) {
            return Ok(*id);
        }
        let id = FunctionId(
            u32::try_from(self.symbols.functions.len())
                .map_err(|_| self.error(ParseErrorKind::ResourceLimit))?,
        );
        self.symbols.functions.push(value);
        self.symbols.function_ids.insert(value, id);
        Ok(id)
    }

    fn current_function(&mut self) -> Result<FunctionId, ParseError> {
        if let Some(id) = self.function {
            return Ok(id);
        }
        let id = self.function_id(Function {
            object: self.object,
            file: self.definition_file,
            name: self.function_name,
        })?;
        self.function = Some(id);
        Ok(id)
    }

    fn number(&self, text: &str) -> Result<u64, ParseError> {
        lex::number(text).map_err(|kind| self.error(kind))
    }

    // File tags share a namespace, as do fn/cfn/jfn and ob/cob.
    // These are format aliases, not our interner or FunctionId values.
    fn name(&mut self, namespace: usize, text: &str) -> Result<StringId, ParseError> {
        let text = text.trim();
        if text.starts_with('(') && text.as_bytes().get(1).is_some_and(u8::is_ascii_digit) {
            let end = text
                .find(')')
                .ok_or_else(|| self.error(ParseErrorKind::InvalidName))?;
            let number = self.number(&text[1..end])?;
            let rest = &text[end + 1..];
            if rest.is_empty() {
                return self.names[namespace]
                    .get(&number)
                    .copied()
                    .ok_or_else(|| self.error(ParseErrorKind::UnknownNameId));
            }
            if !rest.starts_with([' ', '\t']) {
                return Err(self.error(ParseErrorKind::InvalidName));
            }
            let name = rest.trim();
            if namespace == 2
                && name.starts_with('(')
                && name.as_bytes().get(1).is_some_and(u8::is_ascii_digit)
            {
                // --compress-mangled names encode nested context references.
                // Reject until audited; do not present their IDs as real names.
                return Err(self.error(ParseErrorKind::UnsupportedExtension));
            }
            let value = self.intern(name)?;
            self.names[namespace].insert(number, value);
            Ok(value)
        } else {
            self.intern(text)
        }
    }

    fn normalize(&self, mut values: Vec<u64>) -> Result<Costs, ParseError> {
        if values.len() > self.part.events.len() {
            return Err(self.error(ParseErrorKind::InvalidCostWidth));
        }
        values.resize(self.part.events.len(), 0);
        Ok(values.into_boxed_slice())
    }

    fn costs(&self, text: &str) -> Result<Vec<u64>, ParseError> {
        text.split_ascii_whitespace()
            .map(|s| self.number(s))
            .collect()
    }

    fn start_part(&mut self) -> Result<(), ParseError> {
        if self.started {
            return Ok(());
        }
        if self.part.events.is_empty() {
            return Err(self.error(ParseErrorKind::MissingEvents));
        }
        if let Some(summary) = self.part.summary.take() {
            self.part.summary = Some(self.normalize(summary.into_vec()).map_err(|mut error| {
                error.line = self.summary_line;
                error
            })?);
        }
        self.previous.resize(self.part.positions.len(), 0);
        self.queue
            .push_back(ParseEvent::PartStart(self.part.clone()));
        self.started = true;
        Ok(())
    }

    fn end_part(&mut self) -> Result<(), ParseError> {
        if let Some((line, _)) = &self.pending {
            return Err(ParseError {
                kind: ParseErrorKind::MissingAssociationCost,
                line: *line,
            });
        }
        self.start_part()?;
        self.queue.push_back(ParseEvent::PartEnd {
            totals: self.totals.take(),
        });
        self.part = PartHeader::default();
        self.part_line = None;
        self.started = false;
        self.previous.clear();
        self.object = None;
        self.file = None;
        self.definition_file = None;
        self.function_name = None;
        self.function = None;
        self.called_object = None;
        self.called_file = None;
        self.called_function = None;
        self.jump_file = None;
        self.jump_function = None;
        Ok(())
    }

    fn event_definition(&mut self, text: &str) -> Result<EventDefinition, ParseError> {
        let (short, long) = text
            .split_once(':')
            .map_or((text, None), |(a, b)| (a, Some(b.trim().to_owned())));
        let (name, formula) = short
            .split_once('=')
            .map_or((short.trim(), None), |(a, b)| (a.trim(), Some(b.trim())));
        let parsed_name = lex::expression(name).map_err(|kind| self.error(kind))?;
        if parsed_name.len() != 1 || parsed_name[0] != (1, name) {
            return Err(self.error(ParseErrorKind::InvalidEventDefinition));
        }
        let mut terms = None;
        if let Some(formula) = formula {
            let parsed = lex::expression(formula).map_err(|kind| self.error(kind))?;
            let mut result = Vec::new();
            for (coefficient, name) in parsed {
                result.push(EventTerm {
                    coefficient,
                    event: self.intern(name)?,
                });
            }
            terms = Some(result);
        }
        Ok(EventDefinition {
            name: self.intern(name)?,
            terms,
            long_name: long,
        })
    }

    fn header_line(&mut self, key: &str, value: &str) -> Result<(), ParseError> {
        if key == "version" || key == "creator" {
            if self.started || self.part_line.is_some() {
                return Err(self.error(ParseErrorKind::InvalidHeader));
            }
            if key == "version" {
                if self.version_seen {
                    return Err(self.error(ParseErrorKind::InvalidHeader));
                }
                let version = self.number(value)?;
                if version > 1 {
                    return Err(self.error(ParseErrorKind::UnsupportedVersion));
                }
                self.header.version = version;
                self.version_seen = true;
            } else {
                if self.header.creator.is_some() {
                    return Err(self.error(ParseErrorKind::InvalidHeader));
                }
                self.header.creator = Some(value.to_owned());
            }
            return Ok(());
        }
        if key == "totals" {
            self.start_part()?;
            if self.totals.is_some() {
                return Err(self.error(ParseErrorKind::InvalidHeader));
            }
            self.totals = Some(self.normalize(self.costs(value)?)?);
            return Ok(());
        }
        if self.started {
            self.end_part()?;
        }
        self.part_line.get_or_insert(self.line);
        match key {
            "pid" => self.part.metadata.pid = Some(self.number(value)?),
            "thread" => self.part.metadata.thread = Some(self.number(value)?),
            "part" => self.part.metadata.part = Some(self.number(value)?),
            "cmd" => self.part.metadata.command = Some(value.to_owned()),
            "desc" => {
                let (kind, description) = value
                    .split_once(':')
                    .ok_or_else(|| self.error(ParseErrorKind::InvalidHeader))?;
                self.part
                    .metadata
                    .descriptions
                    .push((kind.trim().to_owned(), description.trim().to_owned()));
            }
            "positions" => {
                let positions: Vec<_> = value
                    .split_ascii_whitespace()
                    .map(|s| match s {
                        "instr" => Ok(PositionKind::Instruction),
                        "bb" => Ok(PositionKind::BasicBlock),
                        "line" => Ok(PositionKind::Line),
                        _ => Err(self.error(ParseErrorKind::InvalidPositionOrder)),
                    })
                    .collect::<Result<_, _>>()?;
                if positions.is_empty() || positions.windows(2).any(|p| p[0] >= p[1]) {
                    return Err(self.error(ParseErrorKind::InvalidPositionOrder));
                }
                self.part.positions = positions;
            }
            "events" => {
                if !self.part.events.is_empty() {
                    return Err(self.error(ParseErrorKind::DuplicateEvents));
                }
                for name in value.split_ascii_whitespace() {
                    let id = self.intern(name)?;
                    if self.part.events.contains(&id) {
                        return Err(self.error(ParseErrorKind::DuplicateEvents));
                    }
                    self.part.events.push(id);
                }
                if self.part.events.is_empty() {
                    return Err(self.error(ParseErrorKind::MissingEvents));
                }
            }
            "event" => {
                let definition = self.event_definition(value)?;
                self.part.event_definitions.push(definition);
            }
            "summary" => {
                if self.part.summary.is_some() {
                    return Err(self.error(ParseErrorKind::InvalidHeader));
                }
                self.part.summary = Some(self.costs(value)?.into_boxed_slice());
                self.summary_line = self.line;
            }
            _ => self
                .part
                .metadata
                .extensions
                .push((key.to_owned(), value.to_owned())),
        }
        Ok(())
    }

    fn positions(&self, tokens: &[&str]) -> Result<Positions, ParseError> {
        if tokens.len() != self.part.positions.len() {
            return Err(self.error(ParseErrorKind::InvalidPositionWidth));
        }
        tokens
            .iter()
            .zip(&self.previous)
            .map(|(s, previous)| lex::position(s, *previous).map_err(|kind| self.error(kind)))
            .collect()
    }

    fn body_spec(&mut self, key: &str, value: &str) -> Result<(), ParseError> {
        match key {
            "ob" => {
                self.object = Some(self.name(0, value)?);
                self.function = None;
            }
            "fl" => {
                self.file = Some(self.name(1, value)?);
                self.definition_file = self.file;
                self.function = None;
            }
            "fi" | "fe" => self.file = Some(self.name(1, value)?),
            "fn" => {
                self.function_name = Some(self.name(2, value)?);
                self.file = self.definition_file;
                self.function = None;
                self.current_function()?;
            }
            "cob" => self.called_object = Some(self.name(0, value)?),
            "cfi" | "cfl" => self.called_file = Some(self.name(1, value)?),
            "cfn" => {
                let name = self.name(2, value)?;
                self.called_function = Some(self.function_id(Function {
                    object: self.called_object.or(self.object),
                    file: self.called_file.or(self.file),
                    name: Some(name),
                })?);
            }
            "jfi" => self.jump_file = Some(self.name(1, value)?),
            "jfn" => {
                let name = self.name(2, value)?;
                self.jump_function = Some(self.function_id(Function {
                    object: self.object,
                    file: self.jump_file.or(self.file),
                    name: Some(name),
                })?);
            }
            "calls" | "jump" | "jcnd" => {
                let tokens: Vec<_> = value.split_ascii_whitespace().collect();
                let Some(count) = tokens.first() else {
                    return Err(self.error(ParseErrorKind::InvalidNumber));
                };
                let positions = self.positions(&tokens[1..])?;
                let association = if key == "calls" {
                    let function = self
                        .called_function
                        .ok_or_else(|| self.error(ParseErrorKind::MissingCalledFunction))?;
                    let file = self.symbols.function(function).and_then(|f| f.file);
                    Association::Call {
                        target: Location {
                            function,
                            file,
                            positions,
                        },
                        count: self.number(count)?,
                    }
                } else {
                    let (executed, taken) = if key == "jcnd" {
                        // Valgrind emits taken/executed, unlike the manual.
                        let (taken, executed) = count
                            .split_once('/')
                            .ok_or_else(|| self.error(ParseErrorKind::InvalidJumpCounts))?;
                        let executed = self.number(executed)?;
                        let taken = self.number(taken)?;
                        if taken > executed {
                            return Err(self.error(ParseErrorKind::InvalidJumpCounts));
                        }
                        (executed, Some(taken))
                    } else {
                        (self.number(count)?, None)
                    };
                    let function = match self.jump_function {
                        Some(id) => id,
                        None => self.current_function()?,
                    };
                    let file = self.jump_file.or(self.file);
                    Association::Jump {
                        target: Location {
                            function,
                            file,
                            positions,
                        },
                        executed,
                        taken,
                    }
                };
                self.pending = Some((self.line, association));
            }
            _ => return Err(self.error(ParseErrorKind::UnknownBodyLine)),
        }
        Ok(())
    }

    fn cost_line(&mut self, line: &str) -> Result<(), ParseError> {
        let tokens: Vec<_> = line.split_ascii_whitespace().collect();
        let width = self.part.positions.len();
        if tokens.len() < width {
            return Err(self.error(ParseErrorKind::InvalidPositionWidth));
        }
        let positions = self.positions(&tokens[..width])?;
        let values: Vec<_> = tokens[width..]
            .iter()
            .map(|s| self.number(s))
            .collect::<Result<_, _>>()?;
        let source = Location {
            function: self.current_function()?,
            file: self.file,
            positions: positions.clone(),
        };
        let (line, record) = match self.pending.take() {
            Some((line, Association::Call { target, count })) => {
                let costs = self.normalize(values)?;
                self.called_object = None;
                self.called_file = None;
                (
                    line,
                    Record::Call {
                        source,
                        target,
                        count,
                        costs,
                    },
                )
            }
            Some((
                line,
                Association::Jump {
                    target,
                    executed,
                    taken,
                },
            )) => {
                if !values.is_empty() {
                    return Err(self.error(ParseErrorKind::InvalidCostWidth));
                }
                self.jump_file = None;
                self.jump_function = None;
                (
                    line,
                    Record::Jump {
                        source,
                        target,
                        executed,
                        taken,
                    },
                )
            }
            None => (
                self.line,
                Record::Cost {
                    location: source,
                    costs: self.normalize(values)?,
                },
            ),
        };
        self.previous = positions;
        self.queue
            .push_back(ParseEvent::Record(SpannedRecord { line, record }));
        Ok(())
    }

    fn process(&mut self, input: &str) -> Result<(), ParseError> {
        let input = input.trim_matches([' ', '\t', '\r', '\n']);
        if self.line == 1 && input == "# callgrind format" {
            self.header.has_format_marker = true;
        }
        if input.is_empty() || input.starts_with('#') {
            return Ok(());
        }
        if input.contains('\0') {
            return Err(self.error(ParseErrorKind::InvalidName));
        }
        let numeric =
            input.starts_with(|c: char| c.is_ascii_digit() || matches!(c, '+' | '-' | '*'));
        if let Some((line, _)) = &self.pending
            && !numeric
        {
            return Err(ParseError {
                kind: ParseErrorKind::MissingAssociationCost,
                line: *line,
            });
        }
        // Choose the first separator; names can contain either ':' or '='.
        let separator = input.find([':', '=']);
        if !numeric && let Some(index) = separator {
            let (key, rest) = input.split_at(index);
            if let Some(value) = rest.strip_prefix(':') {
                return self.header_line(key.trim(), value.trim());
            }
            if self.totals.is_some() {
                return Err(self.error(ParseErrorKind::UnexpectedRecord));
            }
            self.start_part()?;
            return self.body_spec(key.trim(), rest[1..].trim());
        }
        if !numeric {
            return Err(self.error(ParseErrorKind::UnknownBodyLine));
        }
        if self.totals.is_some() {
            return Err(self.error(ParseErrorKind::UnexpectedRecord));
        }
        self.start_part()?;
        self.cost_line(input)
    }
}

impl<R: BufRead> Iterator for Decoder<R> {
    type Item = Result<ParseEvent, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(event) = self.queue.pop_front() {
                return Some(Ok(event));
            }
            if self.done {
                return None;
            }
            let mut buffer = std::mem::take(&mut self.buffer);
            buffer.clear();
            const MAX_LINE_BYTES: u64 = 8 * 1024 * 1024;
            let read = self
                .reader
                .by_ref()
                .take(MAX_LINE_BYTES + 1)
                .read_line(&mut buffer);
            let result = match read {
                Ok(0) => {
                    self.done = true;
                    self.end_part()
                }
                Ok(length) => {
                    self.line += 1;
                    if length as u64 > MAX_LINE_BYTES {
                        Err(self.error(ParseErrorKind::ResourceLimit))
                    } else {
                        self.process(&buffer)
                    }
                }
                Err(error) => {
                    self.line += 1;
                    Err(
                        self.error(if error.kind() == std::io::ErrorKind::InvalidData {
                            ParseErrorKind::InvalidUtf8
                        } else {
                            ParseErrorKind::Io
                        }),
                    )
                }
            };
            self.buffer = buffer;
            if let Err(error) = result {
                self.done = true;
                self.queue.clear();
                return Some(Err(error));
            }
        }
    }
}

impl<R: BufRead> std::iter::FusedIterator for Decoder<R> {}

/// Collect incremental events into owned semantic data for interactive use.
pub fn parse_reader(reader: impl BufRead) -> Result<Profile, ParseError> {
    let mut decoder = Decoder::new(reader);
    let mut parts: Vec<Part> = Vec::new();
    for event in decoder.by_ref() {
        match event? {
            ParseEvent::PartStart(header) => parts.push(Part {
                header,
                records: Vec::new(),
                totals: None,
            }),
            ParseEvent::Record(record) => parts
                .last_mut()
                .expect("decoder starts parts before records")
                .records
                .push(record),
            ParseEvent::PartEnd { totals } => {
                parts
                    .last_mut()
                    .expect("decoder starts parts before ending")
                    .totals = totals
            }
        }
    }
    Ok(Profile {
        header: decoder.header,
        symbols: decoder.symbols.freeze(),
        parts,
    })
}

pub fn parse_profile(input: &str) -> Result<Profile, ParseError> {
    parse_reader(std::io::Cursor::new(input.as_bytes()))
}
