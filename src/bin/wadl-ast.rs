/// Generate rust code from wadl
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    input: PathBuf,
}

fn main() {
    env_logger::init();
    let input = Args::parse().input;

    let app: wadl::ast::Application = wadl::parse_file(input).unwrap();

    println!("{:#?}", app);
}
