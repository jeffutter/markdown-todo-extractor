use clap::Parser;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Commandline Args
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    Ok(())
}
