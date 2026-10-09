//! JSON-lines acceptance oracle over the same typed adapter used by the Wasm exports.

use std::error::Error;
use std::io::{self, BufRead, Write};

use qti_wasm::{ConvertRequest, PackageInput};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "camelCase", deny_unknown_fields)]
enum OracleRequest {
    Convert { request: ConvertRequest },
    CheckPackage { input: PackageInput },
    Formats,
}

fn main() -> Result<(), Box<dyn Error>> {
    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    for line in stdin.lock().lines() {
        let request: OracleRequest = serde_json::from_str(&line?)?;
        match request {
            OracleRequest::Convert { request } => {
                // Parity requests set the document date explicitly in both runtimes.
                serde_json::to_writer(
                    &mut stdout,
                    &qti_wasm::convert_request(request, "2000-01-01"),
                )?;
            }
            OracleRequest::CheckPackage { input } => {
                serde_json::to_writer(&mut stdout, &qti_wasm::check_package_request(input))?;
            }
            OracleRequest::Formats => {
                serde_json::to_writer(&mut stdout, &qti_wasm::format_inventory())?;
            }
        }
        writeln!(stdout)?;
    }
    Ok(())
}
