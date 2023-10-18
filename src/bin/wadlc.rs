/// Generate rust code from wadl
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    input: PathBuf,
    output: Option<PathBuf>,
}

fn main() {
    env_logger::init();
    let input = Args::parse().input;
    let output = Args::parse().output;

    let input: wadl::ast::Application = wadl::parse_file(input).unwrap();

    let code = wadl::codegen::generate(&input);

    // If output isn't specified, write to stdout
    if let Some(output) = output {
        std::fs::write(output, code).unwrap();
    } else {
        println!("{}", code);
    }
}
