//! Test adapter for the independent upstream reader; writes only gzip to stdout.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile = pprof_profile::read(std::io::stdin().lock(), pprof_profile::DEFAULT_MAX_BYTES)?;
    pprof_profile::write(&profile, std::io::stdout().lock())?;
    Ok(())
}
