fn main() {
    let header = callgrind_parser::parse_header("version: 1\nevents: Ir\n");
    println!("textgrind stub (recognized {} event)", header.events.len());
}
