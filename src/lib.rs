use clap::Parser;

pub mod config;

/// Retort: An AI pair programmer
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The prompt to send to the model, bypassing the prompt-builder
    #[arg(short, long)]
    prompt: String,
}

pub fn run() -> anyhow::Result<()> {
    let _args = Args::parse();
    let _config = config::load()?;
    Ok(())
}
