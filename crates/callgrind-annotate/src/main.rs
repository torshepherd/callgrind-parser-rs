use callgrind_annotate::{Analysis, Format, Options, Selection, render, render_tsv, select_part};
use clap::Parser;
use std::{
    fs::File,
    io::{self, BufReader, Write},
    process::ExitCode,
};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::parse();
    let profile = callgrind_parser::parse_reader(BufReader::new(File::open(&options.profile)?))?;
    let part = select_part(&profile, options.part)?;
    let analysis = Analysis::build_grouped(&profile, part, options.grouping)?;
    let selected = Selection::build(&analysis, &options)?;
    let output = match options.format {
        Format::Text => render(&profile, part, &analysis, &selected, &options)?,
        Format::Tsv => render_tsv(&analysis, &selected, &options),
    };
    for warning in &analysis.warnings {
        eprintln!("warning: {warning}");
    }
    match io::stdout().lock().write_all(output.as_bytes()) {
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        other => other.map_err(Into::into),
    }
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("callgrind-annotate: {error}");
            ExitCode::FAILURE
        }
    }
}
