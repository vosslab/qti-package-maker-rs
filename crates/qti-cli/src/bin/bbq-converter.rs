//! Process adapter for the BBQ conversion command.

use clap::Parser;

fn main() {
    let arguments = qti_cli::BbqConverterArgs::parse();
    match qti_cli::run_bbq_converter(arguments) {
        Ok(report) => {
            // Python prints this identity before dispatching writers.  It is deliberately
            // independent of -q/-v; those flags govern writer progress, not the receipt.
            println!("Content Name: {}", report.content_name);
            for progress in report.progress {
                println!("{progress}");
            }
            for warning in report.warnings {
                eprintln!(
                    "WARNING: engine '{}' item {}: src '{}' resolved to '{}' was {:?}: {}",
                    warning.engine_name,
                    warning.item_crc,
                    warning.src,
                    warning.resolved,
                    warning.action,
                    warning.reason,
                );
            }
            println!(
                "DONE, saved {} of {} output files",
                report.attempted_outputs, report.attempted_outputs
            );
        }
        Err(error) => {
            eprintln!("ERROR: {error}");
            std::process::exit(error.exit_code());
        }
    }
}
