use pyo3::prelude::*;

/// A sample parser function that will be exposed to Python
#[pyfunction]
fn parse_something(input: &str) -> PyResult<String> {
    Ok(format!("Parsed: {}", input))
}

/// A Python module implemented in Rust.
#[pymodule]
fn parser(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_something, m)?)?;
    Ok(())
} 