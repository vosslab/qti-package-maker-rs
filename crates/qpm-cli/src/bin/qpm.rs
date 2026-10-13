//! Process adapter for package inspection commands.

use clap::Parser;

fn main() {
    let args = qpm_cli::PackageMakerArgs::parse();
    let is_failing_check = matches!(args.command, qpm_cli::PackageMakerCommand::Check { ref path } if qpm_cli::package_check_failed(path));
    match qpm_cli::run_package_maker(args) {
        Ok(output) => {
            println!("{output}");
            if is_failing_check {
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("ERROR: {error}");
            std::process::exit(error.exit_code());
        }
    }
}
