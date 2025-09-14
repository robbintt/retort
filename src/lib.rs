use clap::Parser;

pub mod config;
pub mod db;

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
    let config = config::load()?;
    let expanded_path = shellexpand::tilde(&config.database_path);
    let _conn = db::setup(&expanded_path)?;
    Ok(())
}
