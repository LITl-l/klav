use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

/// Dictionary generation tool for Klav.
///
/// Generates steno dictionaries by mapping kana readings to steno codes.
/// Steno codes follow the StenoWord convention used by klav-core's JapaneseTheory.
///
/// Steno string format (BTreeSet<StenoKey> display order):
///   Left consonants: S, T, K, P, W, H, R
///   Vowels: A, O, E, U
///   Right consonants: -F, -P, -L, -T, -D, -R, -B, -G, -S, -Z
///   Modifiers: *, #V (voiced), #H (half-voiced)
#[derive(Parser)]
#[command(name = "klav-dict-gen", about = "Generate Klav steno dictionaries")]
struct Cli {
    /// Output JSON dictionary path.
    #[arg(short, long, default_value = "dict_generated.json")]
    output: PathBuf,

    /// Input word list (TSV: word\treading per line). Uses built-in list if omitted.
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Include only single-stroke entries (no multi-stroke words).
    #[arg(long)]
    single_only: bool,
}

/// Map a single kana character to its steno string.
/// Returns None for unmappable characters.
fn kana_to_steno(kana: &str) -> Option<&'static str> {
    Some(match kana {
        // 母音 (vowels)
        "あ" => "A",
        "い" => "AE",
        "う" => "U",
        "え" => "E",
        "お" => "O",

        // か行 (K)
        "か" => "KA",
        "き" => "KAE",
        "く" => "KU",
        "け" => "KE",
        "こ" => "KO",

        // さ行 (S)
        "さ" => "SA",
        "し" => "SAE",
        "す" => "SU",
        "せ" => "SE",
        "そ" => "SO",

        // た行 (T)
        "た" => "TA",
        "ち" => "TAE",
        "つ" => "TU",
        "て" => "TE",
        "と" => "TO",

        // な行 (P+H = N consonant)
        "な" => "PHA",
        "に" => "PHAE",
        "ぬ" => "PHU",
        "ね" => "PHE",
        "の" => "PHO",

        // は行 (H)
        "は" => "HA",
        "ひ" => "HAE",
        "ふ" => "HU",
        "へ" => "HE",
        "ほ" => "HO",

        // ま行 (P = M consonant)
        "ま" => "PA",
        "み" => "PAE",
        "む" => "PU",
        "め" => "PE",
        "も" => "PO",

        // や行 (W = Y consonant)
        "や" => "WA",
        "ゆ" => "WU",
        "よ" => "WO",

        // ら行 (R)
        "ら" => "RA",
        "り" => "RAE",
        "る" => "RU",
        "れ" => "RE",
        "ろ" => "RO",

        // わ行 (S+W = W consonant)
        "わ" => "SWA",
        "を" => "SWO",

        // ん (P+H without vowel)
        "ん" => "PH",

        // 濁音 (voiced: #V modifier)
        "が" => "KA#V",
        "ぎ" => "KAE#V",
        "ぐ" => "KU#V",
        "げ" => "KE#V",
        "ご" => "KO#V",

        "ざ" => "SA#V",
        "じ" => "SAE#V",
        "ず" => "SU#V",
        "ぜ" => "SE#V",
        "ぞ" => "SO#V",

        "だ" => "TA#V",
        "ぢ" => "TAE#V",
        "づ" => "TU#V",
        "で" => "TE#V",
        "ど" => "TO#V",

        "ば" => "HA#V",
        "び" => "HAE#V",
        "ぶ" => "HU#V",
        "べ" => "HE#V",
        "ぼ" => "HO#V",

        // 半濁音 (half-voiced: #H modifier)
        "ぱ" => "HA#H",
        "ぴ" => "HAE#H",
        "ぷ" => "HU#H",
        "ぺ" => "HE#H",
        "ぽ" => "HO#H",

        // 拗音 (yōon: * modifier)
        "きゃ" => "KA*",
        "きゅ" => "KU*",
        "きょ" => "KO*",
        "しゃ" => "SA*",
        "しゅ" => "SU*",
        "しょ" => "SO*",
        "ちゃ" => "TA*",
        "ちゅ" => "TU*",
        "ちょ" => "TO*",
        "にゃ" => "PHA*",
        "にゅ" => "PHU*",
        "にょ" => "PHO*",
        "ひゃ" => "HA*",
        "ひゅ" => "HU*",
        "ひょ" => "HO*",
        "みゃ" => "PA*",
        "みゅ" => "PU*",
        "みょ" => "PO*",
        "りゃ" => "RA*",
        "りゅ" => "RU*",
        "りょ" => "RO*",

        // 濁音拗音
        "ぎゃ" => "KA*#V",
        "ぎゅ" => "KU*#V",
        "ぎょ" => "KO*#V",
        "じゃ" => "SA*#V",
        "じゅ" => "SU*#V",
        "じょ" => "SO*#V",
        "ぢゃ" => "TA*#V",
        "ぢゅ" => "TU*#V",
        "ぢょ" => "TO*#V",
        "びゃ" => "HA*#V",
        "びゅ" => "HU*#V",
        "びょ" => "HO*#V",

        // 半濁音拗音
        "ぴゃ" => "HA*#H",
        "ぴゅ" => "HU*#H",
        "ぴょ" => "HO*#H",

        // 促音 (sokuon: -F modifier, standalone)
        "っ" => "-F",

        // 長音 (chōon: -S modifier, standalone)
        "ー" => "-S",

        _ => return None,
    })
}

/// Split a kana reading into individual syllable units.
/// Handles yōon (2-char contracted syllables), sokuon, and regular kana.
fn split_kana(reading: &str) -> Vec<&str> {
    let chars: Vec<char> = reading.chars().collect();
    let mut syllables = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        // Check for 2-character yōon (e.g., きゃ, しゅ, ちょ)
        if i + 1 < chars.len() {
            let two_char = &reading[char_byte_offset(reading, i)..char_byte_offset(reading, i + 2)];
            if kana_to_steno(two_char).is_some() {
                syllables.push(two_char);
                i += 2;
                continue;
            }
        }

        // Single character
        let one_char = &reading[char_byte_offset(reading, i)..char_byte_offset(reading, i + 1)];
        syllables.push(one_char);
        i += 1;
    }

    syllables
}

/// Get the byte offset of the nth character in a string.
fn char_byte_offset(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(offset, _)| offset)
        .unwrap_or(s.len())
}

/// Convert a kana reading to a multi-stroke steno string (joined with "/").
fn reading_to_steno(reading: &str) -> Option<String> {
    let syllables = split_kana(reading);
    let mut steno_parts = Vec::new();

    for syllable in &syllables {
        let steno = kana_to_steno(syllable)?;
        steno_parts.push(steno);
    }

    if steno_parts.is_empty() {
        return None;
    }

    Some(steno_parts.join("/"))
}

/// Built-in Japanese word list: (word, reading) pairs.
/// Sorted roughly by frequency. These are common words useful for steno input.
fn builtin_word_list() -> Vec<(&'static str, &'static str)> {
    vec![
        // === Particles & basic grammar ===
        ("は", "は"),
        ("の", "の"),
        ("に", "に"),
        ("を", "を"),
        ("が", "が"),
        ("で", "で"),
        ("と", "と"),
        ("も", "も"),
        ("か", "か"),
        ("た", "た"),
        ("な", "な"),
        ("て", "て"),

        // === Demonstratives (こそあど) ===
        ("これ", "これ"),
        ("それ", "それ"),
        ("あれ", "あれ"),
        ("どれ", "どれ"),
        ("この", "この"),
        ("その", "その"),
        ("あの", "あの"),
        ("どの", "どの"),
        ("ここ", "ここ"),
        ("そこ", "そこ"),
        ("あそこ", "あそこ"),
        ("どこ", "どこ"),
        ("こう", "こう"),
        ("そう", "そう"),
        ("ああ", "ああ"),
        ("どう", "どう"),

        // === Interrogatives ===
        ("なに", "なに"),
        ("だれ", "だれ"),
        ("いつ", "いつ"),
        ("なぜ", "なぜ"),
        ("いくつ", "いくつ"),
        ("いくら", "いくら"),
        ("どちら", "どちら"),
        ("どんな", "どんな"),

        // === Pronouns ===
        ("わたし", "わたし"),
        ("あなた", "あなた"),
        ("かれ", "かれ"),
        ("かのじょ", "かのじょ"),
        ("わたしたち", "わたしたち"),

        // === Copula & auxiliaries ===
        ("です", "です"),
        ("ます", "ます"),
        ("でした", "でした"),
        ("ました", "ました"),
        ("ない", "ない"),
        ("たい", "たい"),
        ("ある", "ある"),
        ("いる", "いる"),
        ("する", "する"),
        ("なる", "なる"),
        ("できる", "できる"),
        ("くる", "くる"),

        // === Conjunctions & adverbs ===
        ("しかし", "しかし"),
        ("そして", "そして"),
        ("でも", "でも"),
        ("また", "また"),
        ("まだ", "まだ"),
        ("もう", "もう"),
        ("とても", "とても"),
        ("すこし", "すこし"),
        ("たくさん", "たくさん"),
        ("いつも", "いつも"),
        ("ときどき", "ときどき"),
        ("ぜんぶ", "ぜんぶ"),
        ("だけ", "だけ"),
        ("ほど", "ほど"),
        ("まで", "まで"),
        ("から", "から"),
        ("より", "より"),
        ("ほか", "ほか"),
        ("けど", "けど"),
        ("ので", "ので"),
        ("のに", "のに"),
        ("ため", "ため"),
        ("ながら", "ながら"),
        ("たら", "たら"),
        ("なら", "なら"),
        ("ほんとうに", "ほんとうに"),
        ("やはり", "やはり"),
        ("たぶん", "たぶん"),
        ("きっと", "きっと"),
        ("ちょっと", "ちょっと"),
        ("すぐ", "すぐ"),
        ("もっと", "もっと"),

        // === Greetings & phrases ===
        ("こんにちは", "こんにちは"),
        ("こんばんは", "こんばんは"),
        ("おはよう", "おはよう"),
        ("さようなら", "さようなら"),
        ("ありがとう", "ありがとう"),
        ("すみません", "すみません"),
        ("おねがいします", "おねがいします"),
        ("おめでとう", "おめでとう"),
        ("いただきます", "いただきます"),
        ("ごちそうさま", "ごちそうさま"),

        // === Common nouns ===
        ("ひと", "ひと"),
        ("もの", "もの"),
        ("こと", "こと"),
        ("ところ", "ところ"),
        ("とき", "とき"),
        ("ひ", "ひ"),
        ("ほん", "ほん"),
        ("みず", "みず"),
        ("やま", "やま"),
        ("かわ", "かわ"),
        ("うみ", "うみ"),
        ("そら", "そら"),
        ("はな", "はな"),
        ("き", "き"),
        ("いえ", "いえ"),
        ("まち", "まち"),
        ("くに", "くに"),
        ("みち", "みち"),
        ("くるま", "くるま"),
        ("でんしゃ", "でんしゃ"),
        ("えき", "えき"),
        ("がっこう", "がっこう"),
        ("びょういん", "びょういん"),
        ("しごと", "しごと"),
        ("かいしゃ", "かいしゃ"),
        ("おかね", "おかね"),
        ("じかん", "じかん"),
        ("あさ", "あさ"),
        ("ひる", "ひる"),
        ("よる", "よる"),
        ("きょう", "きょう"),
        ("あした", "あした"),
        ("きのう", "きのう"),
        ("いま", "いま"),
        ("せんしゅう", "せんしゅう"),
        ("らいしゅう", "らいしゅう"),
        ("ことし", "ことし"),
        ("きょねん", "きょねん"),
        ("らいねん", "らいねん"),
        ("ちち", "ちち"),
        ("はは", "はは"),
        ("あに", "あに"),
        ("あね", "あね"),
        ("おとうと", "おとうと"),
        ("いもうと", "いもうと"),
        ("ともだち", "ともだち"),
        ("せんせい", "せんせい"),
        ("しんぶん", "しんぶん"),
        ("てがみ", "てがみ"),
        ("でんわ", "でんわ"),
        ("たべもの", "たべもの"),
        ("のみもの", "のみもの"),
        ("ごはん", "ごはん"),
        ("おちゃ", "おちゃ"),
        ("にく", "にく"),
        ("さかな", "さかな"),
        ("やさい", "やさい"),
        ("くだもの", "くだもの"),
        ("からだ", "からだ"),
        ("あたま", "あたま"),
        ("め", "め"),
        ("みみ", "みみ"),
        ("くち", "くち"),
        ("て", "て"),
        ("あし", "あし"),
        ("こころ", "こころ"),
        ("きもち", "きもち"),
        ("かんがえ", "かんがえ"),
        ("ことば", "ことば"),

        // === Common verbs (dictionary form) ===
        ("みる", "みる"),
        ("きく", "きく"),
        ("はなす", "はなす"),
        ("かく", "かく"),
        ("よむ", "よむ"),
        ("たべる", "たべる"),
        ("のむ", "のむ"),
        ("いく", "いく"),
        ("かえる", "かえる"),
        ("おきる", "おきる"),
        ("ねる", "ねる"),
        ("あそぶ", "あそぶ"),
        ("はたらく", "はたらく"),
        ("べんきょうする", "べんきょうする"),
        ("しる", "しる"),
        ("おもう", "おもう"),
        ("わかる", "わかる"),
        ("つかう", "つかう"),
        ("つくる", "つくる"),
        ("もつ", "もつ"),
        ("おく", "おく"),
        ("とる", "とる"),
        ("だす", "だす"),
        ("いう", "いう"),
        ("おしえる", "おしえる"),
        ("まなぶ", "まなぶ"),
        ("あるく", "あるく"),
        ("はしる", "はしる"),
        ("およぐ", "およぐ"),
        ("まつ", "まつ"),
        ("あう", "あう"),

        // === i-adjectives ===
        ("おおきい", "おおきい"),
        ("ちいさい", "ちいさい"),
        ("たかい", "たかい"),
        ("やすい", "やすい"),
        ("ながい", "ながい"),
        ("みじかい", "みじかい"),
        ("あたらしい", "あたらしい"),
        ("ふるい", "ふるい"),
        ("いい", "いい"),
        ("わるい", "わるい"),
        ("おいしい", "おいしい"),
        ("はやい", "はやい"),
        ("おそい", "おそい"),
        ("つよい", "つよい"),
        ("よわい", "よわい"),
        ("あつい", "あつい"),
        ("さむい", "さむい"),
        ("あかるい", "あかるい"),
        ("くらい", "くらい"),
        ("うれしい", "うれしい"),
        ("かなしい", "かなしい"),
        ("たのしい", "たのしい"),
        ("むずかしい", "むずかしい"),
        ("やさしい", "やさしい"),

        // === na-adjectives (stem) ===
        ("きれい", "きれい"),
        ("しずか", "しずか"),
        ("げんき", "げんき"),
        ("すき", "すき"),
        ("きらい", "きらい"),
        ("じょうず", "じょうず"),
        ("へた", "へた"),
        ("だいじ", "だいじ"),
        ("たいせつ", "たいせつ"),
        ("ひつよう", "ひつよう"),
        ("かんたん", "かんたん"),
        ("ふくざつ", "ふくざつ"),
        ("べんり", "べんり"),
        ("ゆうめい", "ゆうめい"),

        // === Numbers ===
        ("いち", "いち"),
        ("に", "に"),
        ("さん", "さん"),
        ("し", "し"),
        ("ご", "ご"),
        ("ろく", "ろく"),
        ("なな", "なな"),
        ("はち", "はち"),
        ("きゅう", "きゅう"),
        ("じゅう", "じゅう"),
        ("ひゃく", "ひゃく"),
        ("せん", "せん"),
        ("まん", "まん"),

        // === Counters & time ===
        ("ねん", "ねん"),
        ("がつ", "がつ"),
        ("にち", "にち"),
        ("じ", "じ"),
        ("ふん", "ふん"),
        ("びょう", "びょう"),

        // === Common endings ===
        ("ている", "ている"),
        ("てある", "てある"),
        ("てくる", "てくる"),
        ("ていく", "ていく"),
        ("てみる", "てみる"),
        ("てほしい", "てほしい"),
        ("ことができる", "ことができる"),
        ("なければならない", "なければならない"),

        // === Useful phrases ===
        ("だいじょうぶ", "だいじょうぶ"),
        ("もちろん", "もちろん"),
        ("ぜんぜん", "ぜんぜん"),
        ("なるほど", "なるほど"),
        ("そうですね", "そうですね"),
        ("おつかれさま", "おつかれさま"),
    ]
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let word_list: Vec<(String, String)> = if let Some(ref input_path) = cli.input {
        // Read TSV word list: word\treading per line
        let content = std::fs::read_to_string(input_path)
            .with_context(|| format!("failed to read {}", input_path.display()))?;

        content
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 2 {
                    Some((parts[0].to_string(), parts[1].to_string()))
                } else {
                    eprintln!("skipping malformed line: {line}");
                    None
                }
            })
            .collect()
    } else {
        builtin_word_list()
            .into_iter()
            .map(|(w, r)| (w.to_string(), r.to_string()))
            .collect()
    };

    let mut dict = BTreeMap::new();
    let mut skipped = 0;
    let mut single_stroke = 0;
    let mut multi_stroke = 0;

    for (word, reading) in &word_list {
        match reading_to_steno(reading) {
            Some(steno) => {
                let is_multi = steno.contains('/');

                if cli.single_only && is_multi {
                    continue;
                }

                if is_multi {
                    multi_stroke += 1;
                } else {
                    single_stroke += 1;
                }

                // Only insert if no conflict (first entry wins)
                dict.entry(steno).or_insert_with(|| word.clone());
            }
            None => {
                eprintln!("could not convert: {word} ({reading})");
                skipped += 1;
            }
        }
    }

    let json = serde_json::to_string_pretty(&dict)?;
    std::fs::write(&cli.output, &json)
        .with_context(|| format!("failed to write {}", cli.output.display()))?;

    println!(
        "wrote {} entries to {} ({} single-stroke, {} multi-stroke, {} skipped)",
        dict.len(),
        cli.output.display(),
        single_stroke,
        multi_stroke,
        skipped
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_vowel_steno() {
        assert_eq!(kana_to_steno("あ"), Some("A"));
        assert_eq!(kana_to_steno("い"), Some("AE"));
        assert_eq!(kana_to_steno("う"), Some("U"));
        assert_eq!(kana_to_steno("え"), Some("E"));
        assert_eq!(kana_to_steno("お"), Some("O"));
    }

    #[test]
    fn consonant_vowel_steno() {
        assert_eq!(kana_to_steno("か"), Some("KA"));
        assert_eq!(kana_to_steno("し"), Some("SAE"));
        assert_eq!(kana_to_steno("の"), Some("PHO"));
        assert_eq!(kana_to_steno("も"), Some("PO"));
    }

    #[test]
    fn voiced_steno() {
        assert_eq!(kana_to_steno("が"), Some("KA#V"));
        assert_eq!(kana_to_steno("で"), Some("TE#V"));
        assert_eq!(kana_to_steno("ぼ"), Some("HO#V"));
    }

    #[test]
    fn yoon_steno() {
        assert_eq!(kana_to_steno("きゃ"), Some("KA*"));
        assert_eq!(kana_to_steno("しゅ"), Some("SU*"));
        assert_eq!(kana_to_steno("ちょ"), Some("TO*"));
    }

    #[test]
    fn split_kana_simple() {
        assert_eq!(split_kana("かた"), vec!["か", "た"]);
        assert_eq!(split_kana("あいう"), vec!["あ", "い", "う"]);
    }

    #[test]
    fn split_kana_yoon() {
        assert_eq!(split_kana("きょう"), vec!["きょ", "う"]);
        assert_eq!(split_kana("しゃしん"), vec!["しゃ", "し", "ん"]);
    }

    #[test]
    fn reading_to_steno_simple() {
        assert_eq!(reading_to_steno("か"), Some("KA".into()));
        assert_eq!(reading_to_steno("かた"), Some("KA/TA".into()));
    }

    #[test]
    fn reading_to_steno_complex() {
        assert_eq!(reading_to_steno("きょう"), Some("KO*/U".into()));
        assert_eq!(reading_to_steno("がっこう"), Some("KA#V/-F/KO/U".into()));
    }

    #[test]
    fn reading_to_steno_n() {
        assert_eq!(reading_to_steno("ほん"), Some("HO/PH".into()));
        assert_eq!(reading_to_steno("にほん"), Some("PHAE/HO/PH".into()));
    }
}
