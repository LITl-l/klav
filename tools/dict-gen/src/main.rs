use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

/// Dictionary generation tool for Klav.
///
/// Generates steno dictionaries from frequency-ranked word lists.
/// Future: integrate with MeCab + UniDic for Japanese word frequency data.
#[derive(Parser)]
#[command(name = "klav-dict-gen", about = "Generate Klav steno dictionaries")]
struct Cli {
    /// Output JSON dictionary path.
    #[arg(short, long, default_value = "dict_generated.json")]
    output: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Phase 0: generate a minimal example dictionary
    let mut dict = BTreeMap::new();

    // Basic particles and common words
    let entries = [
        ("A", "あ"),
        ("AE", "い"),
        ("U", "う"),
        ("E", "え"),
        ("O", "お"),
        ("KA", "か"),
        ("SA", "さ"),
        ("TA", "た"),
        ("NA", "な"),
        ("HA", "は"),
    ];

    for (stroke, word) in entries {
        dict.insert(stroke.to_string(), word.to_string());
    }

    let json = serde_json::to_string_pretty(&dict)?;
    std::fs::write(&cli.output, &json)
        .with_context(|| format!("failed to write {}", cli.output.display()))?;

    println!("wrote {} entries to {}", dict.len(), cli.output.display());

    Ok(())
}
