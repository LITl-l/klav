mod input;
mod ipc;
mod output;

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;

use klav_core::config::Config;
use klav_core::dictionary::{Dictionary, DictionaryStack};
use klav_core::keymap::KeyMap;
use klav_core::stroke::StrokeDetector;
use klav_core::theory::JapaneseTheory;
use klav_core::translator::{TranslationResult, Translator};

use input::InputBackend;
use input::KeyEventKind;
use output::OutputBackend;

#[derive(Parser)]
#[command(name = "klav-daemon", about = "Klav stenotype engine daemon")]
struct Cli {
    /// Path to the main configuration file.
    #[arg(short, long, default_value = "klav.toml")]
    config: PathBuf,

    /// Path to a specific evdev device (auto-detect if omitted).
    #[arg(short, long)]
    device: Option<PathBuf>,

    /// Don't grab the input device (useful for debugging).
    #[arg(long)]
    no_grab: bool,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    let cli = Cli::parse();

    log::info!("klav-daemon starting");

    // Load config
    let config = Config::load(&cli.config)
        .context("failed to load config")?;
    let config_dir = cli.config.parent().unwrap_or_else(|| std::path::Path::new("."));

    // Load keymap
    let keymap_path = Config::resolve_path(config_dir, &config.keymap);
    let keymap = KeyMap::load(&keymap_path)
        .context("failed to load keymap")?;

    // Load theory and dictionaries for default language
    let default_lang = &config.languages.default;
    let lang_config = config.languages.languages.get(default_lang)
        .with_context(|| format!("language '{default_lang}' not found in config"))?;

    // Load theory rules
    let theory_dir = Config::resolve_path(config_dir, &format!("theories/{}", lang_config.theory));
    let rules_path = theory_dir.join("rules.toml");
    let theory = if rules_path.exists() {
        Box::new(JapaneseTheory::load(&rules_path)
            .context("failed to load theory rules")?)
    } else {
        log::warn!("no rules.toml found for theory, using empty theory");
        Box::new(JapaneseTheory::from_toml("[syllable_rules]\n")?)
    };

    // Load dictionaries
    let mut dict_stack = DictionaryStack::new();
    for dict_file in &lang_config.dictionary {
        let dict_path = Config::resolve_path(config_dir, dict_file);
        if dict_path.exists() {
            let dict = Dictionary::load_json(&dict_path)
                .with_context(|| format!("failed to load dictionary: {}", dict_path.display()))?;
            dict_stack.push_back(dict);
        } else {
            log::warn!("dictionary not found: {}", dict_path.display());
        }
    }

    log::info!(
        "loaded language '{}' with {} dictionary entries",
        default_lang,
        dict_stack.total_entries()
    );

    // Initialize translator
    let mut translator = Translator::new(theory, dict_stack);

    // Initialize stroke detector
    let timeout = Duration::from_millis(config.stroke.timeout_ms);
    let mut detector = StrokeDetector::new(timeout);

    // Platform-specific input/output
    run_linux(&cli, keymap, &mut detector, &mut translator)
}

#[cfg(target_os = "linux")]
fn run_linux(
    cli: &Cli,
    keymap: KeyMap,
    detector: &mut StrokeDetector,
    translator: &mut Translator,
) -> Result<()> {
    use input::evdev::EvdevInput;
    use output::uinput::UinputOutput;

    // Open input device
    let mut input = if let Some(ref device_path) = cli.device {
        EvdevInput::open(device_path.clone())?
    } else {
        EvdevInput::auto_detect()?
    };

    log::info!("using input device: {}", input.device_name());

    // Grab the device unless --no-grab
    if !cli.no_grab {
        input.grab().context("failed to grab input device")?;
        log::info!("input device grabbed");
    }

    // Open output
    let mut output = UinputOutput::new()?;

    log::info!("klav-daemon ready — press Ctrl+C to stop");

    // Main loop
    let result = main_loop(&keymap, &mut input, &mut output, detector, translator);

    // Ungrab on exit
    if !cli.no_grab {
        let _ = input.ungrab();
        log::info!("input device released");
    }

    result
}

#[cfg(not(target_os = "linux"))]
fn run_linux(
    _cli: &Cli,
    _keymap: KeyMap,
    _detector: &mut StrokeDetector,
    _translator: &mut Translator,
) -> Result<()> {
    anyhow::bail!("Linux evdev backend is not available on this platform")
}

fn main_loop(
    keymap: &KeyMap,
    input: &mut dyn InputBackend,
    output: &mut dyn OutputBackend,
    detector: &mut StrokeDetector,
    translator: &mut Translator,
) -> Result<()> {
    loop {
        let event = input.next_event()?;

        // Map physical key to steno key
        let Some(steno_key) = keymap.get(event.code) else {
            continue; // Not a mapped key
        };

        let stroke = match event.kind {
            KeyEventKind::Press => {
                detector.key_down(steno_key);
                None
            }
            KeyEventKind::Release => detector.key_up(steno_key),
        };

        if let Some(stroke) = stroke {
            log::debug!("stroke: {stroke}");

            match translator.translate(&stroke) {
                TranslationResult::Output(text) => {
                    output.type_text(&text)?;
                }
                TranslationResult::Undo(count) => {
                    output.backspace(count)?;
                }
                TranslationResult::LangSwitch => {
                    log::info!("language switch requested (not yet implemented)");
                }
                TranslationResult::Nothing => {
                    log::debug!("no translation for stroke: {stroke}");
                }
            }
        }
    }
}
