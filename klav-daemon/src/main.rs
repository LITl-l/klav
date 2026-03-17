mod input;
mod output;

use std::path::{Path, PathBuf};
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

    /// Path to a specific evdev device (Linux only, auto-detect if omitted).
    #[cfg(target_os = "linux")]
    #[arg(short, long)]
    device: Option<PathBuf>,

    /// Don't grab the input device (useful for debugging).
    #[arg(long)]
    no_grab: bool,
}

/// Tracks available languages and handles switching between them.
struct LanguageManager {
    language_names: Vec<String>,
    current_index: usize,
    config_dir: PathBuf,
    config: Config,
}

impl LanguageManager {
    fn new(config: Config, config_dir: PathBuf) -> Result<Self> {
        let language_names: Vec<String> = config.languages.languages.keys().cloned().collect();
        if language_names.is_empty() {
            anyhow::bail!("no languages configured");
        }
        let current_index = language_names.iter()
            .position(|l| l == &config.languages.default)
            .unwrap_or(0);
        Ok(Self { language_names, current_index, config_dir, config })
    }

    fn current_name(&self) -> &str {
        &self.language_names[self.current_index]
    }

    /// Load theory + dictionaries for the current language.
    fn load_current(&self) -> Result<(Box<dyn klav_core::theory::Theory>, DictionaryStack)> {
        let lang_config = &self.config.languages.languages[self.current_name()];
        load_language(&self.config_dir, &lang_config.theory, &lang_config.dictionary)
    }

    /// Switch to the next language and return its name.
    fn advance(&mut self) -> &str {
        self.current_index = (self.current_index + 1) % self.language_names.len();
        self.current_name()
    }
}

fn load_language(
    config_dir: &Path,
    theory_name: &str,
    dict_paths: &[String],
) -> Result<(Box<dyn klav_core::theory::Theory>, DictionaryStack)> {
    let theory_dir = Config::resolve_path(config_dir, &format!("theories/{theory_name}"));
    let rules_path = theory_dir.join("rules.toml");
    let theory: Box<dyn klav_core::theory::Theory> = if rules_path.exists() {
        Box::new(JapaneseTheory::load(&rules_path)
            .context("failed to load theory rules")?)
    } else {
        log::warn!("no rules.toml found for theory '{theory_name}', using empty theory");
        Box::new(JapaneseTheory::from_toml("[syllable_rules]\n")?)
    };

    let mut dict_stack = DictionaryStack::new();
    for dict_file in dict_paths {
        let dict_path = Config::resolve_path(config_dir, dict_file);
        if dict_path.exists() {
            let dict = Dictionary::load_json(&dict_path)
                .with_context(|| format!("failed to load dictionary: {}", dict_path.display()))?;
            dict_stack.push_back(dict);
        } else {
            log::warn!("dictionary not found: {}", dict_path.display());
        }
    }

    Ok((theory, dict_stack))
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    let cli = Cli::parse();

    log::info!("klav-daemon starting");

    // Load config
    let config = Config::load(&cli.config)
        .context("failed to load config")?;
    let config_dir = cli.config.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();

    // Load keymap
    let keymap_path = Config::resolve_path(&config_dir, &config.keymap);
    let keymap = KeyMap::load(&keymap_path)
        .context("failed to load keymap")?;

    // Initialize language manager
    let mut lang_mgr = LanguageManager::new(config, config_dir)?;
    log::info!("default language: {}", lang_mgr.current_name());

    // Load default language
    let (theory, dicts) = lang_mgr.load_current()
        .context("failed to load default language")?;
    log::info!("loaded '{}' ({} dict entries)", lang_mgr.current_name(), dicts.total_entries());

    let mut translator = Translator::new(theory, dicts);

    // Initialize stroke detector
    let timeout = Duration::from_millis(lang_mgr.config.stroke.timeout_ms);
    let mut detector = StrokeDetector::new(timeout);

    // Platform-specific input/output
    run_platform(&cli, keymap, &mut detector, &mut translator, &mut lang_mgr)
}

#[cfg(target_os = "linux")]
fn run_platform(
    cli: &Cli,
    keymap: KeyMap,
    detector: &mut StrokeDetector,
    translator: &mut Translator,
    lang_mgr: &mut LanguageManager,
) -> Result<()> {
    use input::evdev::EvdevInput;

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

    // Open output backend (configurable)
    let backend_name = &lang_mgr.config.output.backend;
    let mut output = output::create_backend(backend_name)
        .context(format!("failed to create output backend '{backend_name}'"))?;

    log::info!("klav-daemon ready — press Ctrl+C to stop");

    // Main loop
    let result = main_loop(&keymap, &mut input, output.as_mut(), detector, translator, lang_mgr);

    // Ungrab on exit
    if !cli.no_grab {
        let _ = input.ungrab();
        log::info!("input device released");
    }

    result
}

#[cfg(target_os = "windows")]
fn run_platform(
    cli: &Cli,
    keymap: KeyMap,
    detector: &mut StrokeDetector,
    translator: &mut Translator,
    lang_mgr: &mut LanguageManager,
) -> Result<()> {
    use input::win32hook::Win32HookInput;

    let mut input = Win32HookInput::new()?;
    log::info!("using Windows low-level keyboard hook");

    if !cli.no_grab {
        input.grab()?;
        log::info!("keyboard input grabbed (keys will be swallowed)");
    }

    let backend_name = &lang_mgr.config.output.backend;
    let mut output = output::create_backend(backend_name)
        .context(format!("failed to create output backend '{backend_name}'"))?;

    log::info!("klav-daemon ready — press Ctrl+C to stop");

    let result = main_loop(&keymap, &mut input, output.as_mut(), detector, translator, lang_mgr);

    if !cli.no_grab {
        let _ = input.ungrab();
        log::info!("keyboard hook released");
    }

    result
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn run_platform(
    _cli: &Cli,
    _keymap: KeyMap,
    _detector: &mut StrokeDetector,
    _translator: &mut Translator,
    _lang_mgr: &mut LanguageManager,
) -> Result<()> {
    anyhow::bail!("no input backend available for this platform")
}

fn main_loop(
    keymap: &KeyMap,
    input: &mut dyn InputBackend,
    output: &mut dyn OutputBackend,
    detector: &mut StrokeDetector,
    translator: &mut Translator,
    lang_mgr: &mut LanguageManager,
) -> Result<()> {
    loop {
        let event = input.next_event()?;

        // Map physical key to steno key
        let Some(steno_key) = keymap.get(event.code) else {
            continue;
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
                TranslationResult::Replace { backspace, text } => {
                    output.backspace(backspace)?;
                    output.type_text(&text)?;
                }
                TranslationResult::Undo(count) => {
                    output.backspace(count)?;
                }
                TranslationResult::LangSwitch => {
                    let new_lang = lang_mgr.advance();
                    log::info!("switching to language: {new_lang}");

                    match lang_mgr.load_current() {
                        Ok((theory, dicts)) => {
                            log::info!("loaded '{}' ({} dict entries)",
                                lang_mgr.current_name(), dicts.total_entries());
                            translator.set_theory(theory);
                            translator.set_dictionaries(dicts);
                        }
                        Err(e) => {
                            log::error!("failed to load language '{}': {e}", lang_mgr.current_name());
                        }
                    }
                }
                TranslationResult::Nothing => {
                    log::debug!("no translation for stroke: {stroke}");
                }
            }
        }
    }
}
