//! Development tooling entry point.
mod crc_corpus;
mod current_python;
mod oracle_crosscheck;
mod parity;
mod table_bench;
mod table_corpus;
mod table_gallery;

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let result = match arguments.first().map(String::as_str) {
        None | Some("--help" | "-h") => {
            println!(
                "Usage: cargo xtask <command>\n\nCommands:\n  crc-corpus    Compare item identities with current Python\n  table-corpus  Harvest real generator tables and canvases\n  table-bench   Measure current Python conversion stages\n  table-gallery  Build the native/Python table review gallery\n  oracle-crosscheck  Compare package integrity with current Python\n  parity        Compare exports with current Python\n\nRun the converter with cargo run -p qti-cli --bin bbq-converter -- --help."
            );
            Ok(())
        }
        Some("table-corpus") => table_corpus::run(&arguments[1..]),
        Some("crc-corpus") => crc_corpus::run(&arguments[1..]),
        Some("oracle-crosscheck") => oracle_crosscheck::run(&arguments[1..]),
        Some("parity") => parity::run(&arguments[1..]),
        Some("table-bench") => table_bench::run(&arguments[1..]),
        Some("table-gallery") => table_gallery::run(&arguments[1..]),
        Some(command) => Err(format!("unknown xtask command: {command}")),
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
