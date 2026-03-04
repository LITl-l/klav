# Klav — ロードマップ

> **Klav** : ラテン語 clavis（鍵）に由来。思考の速度で多言語入力を実現する、軽量・高速なオープンソース ステノタイプエンジン。

---

## 1. プロジェクト概要

### 1.1 ビジョン

思考の速度でタイピングできること。言語の壁を意識せず、頭に浮かんだ言葉がそのまま画面に現れる体験を実現する。

### 1.2 Plover との差別化

| 項目 | Plover | Klav |
|------|--------|------|
| 言語 | Python（CPython） | Rust |
| 起動速度 | 数秒 | 瞬時（ネイティブバイナリ） |
| 入力レイテンシ | アプリケーション層キーフック | evdev/uinput 直接操作 |
| 日本語対応 | プラグインで追加 | コア機能として内蔵 |
| 漢字変換 | IME依存 | 段階的に内蔵（IME → 内蔵変換 → 直接入力） |
| 多言語切替 | System プラグイン切替 | ストローク一発で即座に切替 |
| GUI | Qt (PySide6) | egui（軽量） |
| 配布 | Python + 依存パッケージ | シングルバイナリ |

### 1.3 設計原則

1. **Speed** — ソフトウェアは思考を待たせてはならない
2. **Simplicity** — 設定ファイルとルールベースで動作を理解可能にする
3. **Modularity** — ステノ理論はプラグイン。エンジンは言語に依存しない
4. **Openness** — 独自理論への移行も、既存理論の流用も自由

---

## 2. 技術スタック

### 2.1 コア

| コンポーネント | 選定 | 理由 |
|--------------|------|------|
| 言語 | Rust | GC無し・低レイテンシ・evdevエコシステム充実 |
| 入力取得 (Linux) | evdev crate | カーネル直近でキーイベントを取得 |
| 出力 (Linux) | uinput | 仮想キーボードデバイスとして文字を送出 |
| 設定ファイル | TOML | Rust エコシステムの標準。人間が読み書きしやすい |
| 辞書フォーマット | JSON + バイナリキャッシュ | 互換性（JSON）と速度（バイナリ）の両立 |

### 2.2 GUI

| コンポーネント | 選定 | 理由 |
|--------------|------|------|
| 設定ウィンドウ | egui (eframe) | 軽量・クロスプラットフォーム・Rustネイティブ |
| トレイアイコン | tray-icon crate | Linux/Windows/macOS 対応 |
| デーモン ↔ GUI 通信 | Unix socket (Linux) / Named pipe (Windows) | シンプルなIPC |

### 2.3 対応プラットフォーム（優先順）

1. **Linux** — evdev/uinput。Wayland/X11 両対応
2. **Windows** — Raw Input API / SendInput
3. **macOS** — IOKit HID / CGEvent

---

## 3. アーキテクチャ

```
┌────────────────────────────────────────────────────────┐
│                      Klav System                       │
│                                                        │
│  ┌─────────────┐  IPC   ┌───────────────────────────┐  │
│  │  klav-gui   │◄──────►│      klav-daemon          │  │
│  │  (egui)     │        │                           │  │
│  │  - 設定編集  │        │  ┌─────────────────────┐  │  │
│  │  - 状態表示  │        │  │   Input Layer       │  │  │
│  │  - 辞書管理  │        │  │   evdev → grab      │  │  │
│  └─────────────┘        │  └──────────┬──────────┘  │  │
│                         │             │              │  │
│                         │  ┌──────────▼──────────┐  │  │
│                         │  │   Stroke Detector    │  │  │
│                         │  │   同時押し → コード    │  │  │
│                         │  └──────────┬──────────┘  │  │
│                         │             │              │  │
│                         │  ┌──────────▼──────────┐  │  │
│                         │  │   Theory Engine      │  │  │
│                         │  │   (プラグイン方式)     │  │  │
│                         │  │                      │  │  │
│                         │  │   ┌──────────────┐   │  │  │
│                         │  │   │ Layer 1:     │   │  │  │
│                         │  │   │ ルールベース  │   │  │  │
│                         │  │   │ 音節変換     │   │  │  │
│                         │  │   ├──────────────┤   │  │  │
│                         │  │   │ Layer 2:     │   │  │  │
│                         │  │   │ 単語辞書     │   │  │  │
│                         │  │   ├──────────────┤   │  │  │
│                         │  │   │ Layer 3:     │   │  │  │
│                         │  │   │ 漢字直接入力  │   │  │  │
│                         │  │   └──────────────┘   │  │  │
│                         │  └──────────┬──────────┘  │  │
│                         │             │              │  │
│                         │  ┌──────────▼──────────┐  │  │
│                         │  │   Output Layer       │  │  │
│                         │  │   uinput / IME連携   │  │  │
│                         │  └─────────────────────┘  │  │
│                         └───────────────────────────┘  │
└────────────────────────────────────────────────────────┘
```

### 3.1 Theory Engine の3層構造

#### Layer 1: ルールベース音節変換（基本層）

子音コード × 母音コード → かな音節をアルゴリズムで生成。辞書不要。
これだけ覚えればどんな日本語でも入力可能。最低保証の層。

```
例: 子音[K] + 母音[A] → か
    子音[S] + 母音[I] → し
```

英語の場合: 左バンク（子音）+ 母音 + 右バンク（子音）→ 音節

#### Layer 2: 単語辞書（高速層）

頻出語・フレーズを1ストロークで入力。StenoWord辞書を流用・拡張。
使いながらユーザーが自分の辞書を育てていく。

```
例: [特定コード] → "ありがとうございます"
    [特定コード] → "the"
```

#### Layer 3: 漢字直接入力（最速層）— 将来実装

構造ベースの漢字コード。IME不要で同音異義語の問題を回避。
Frag's Japanese Theory の構造ベース方式を参考に設計。

```
例: [構造コード] → 漢字一文字を直接出力
```

### 3.2 多言語切替

- エンジン内部で理論（Theory）を切替
- 特定のストローク（例: 両手の特定コード同時押し）で即座に切替
- OS側のIME切替とは独立
- 各言語の理論は独立したTOML + 辞書ファイルのセット

```toml
# klav.toml
[languages]
default = "japanese"
switch_stroke = "LANG"  # 言語切替に使うステノキー

[languages.japanese]
theory = "klav-ja-stenoword"
dictionary = ["ja_base.json", "ja_user.json"]

[languages.english]
theory = "klav-en-plover"
dictionary = ["en_base.json", "en_user.json"]
```

### 3.3 キーマップ

物理キー → ステノ論理キーのマッピング。設定ファイルで自由に定義。

```toml
# keymap.toml — QWERTY キーボード用の例
[keymap]
# 左手 子音
"KEY_Q" = "S1"
"KEY_W" = "T1"
"KEY_E" = "P1"
"KEY_R" = "H1"

# 左手 母音（親指）
"KEY_C" = "A"
"KEY_V" = "O"

# 右手 母音（親指）
"KEY_N" = "E"
"KEY_M" = "U"

# 右手 子音
"KEY_U" = "F1"
"KEY_I" = "P2"
"KEY_O" = "L1"
"KEY_P" = "T2"

# 特殊キー
"KEY_SPACE" = "LANG"   # 言語切替
"KEY_BACKSPACE" = "UNDO" # 直前のストローク取り消し
```

---

## 4. 開発フェーズ

### Phase 0: 基盤（v0.1） — 目標: 動く最小構成

**ゴール**: evdev でキーを読み取り、同時押しを検出し、uinput で文字を出力する

- [ ] プロジェクト構造（Cargo workspace）
  - `klav-core` — ストローク検出・理論エンジン・辞書
  - `klav-daemon` — evdev/uinput 統合、メインループ
  - `klav-gui` — 設定ウィンドウ（後回し可）
- [ ] evdev 入力取得（キーボードデバイスの自動検出・grab）
- [ ] ストローク検出（同時押し判定ロジック）
  - 全キーが押され→全キーが離された時点で1ストローク確定
  - タイムアウト設定可能
- [ ] TOML キーマップ読み込み
- [ ] 最小辞書（ハードコード10エントリ程度）
- [ ] uinput でかな文字出力
- [ ] 基本的な undo（直前ストローク取り消し）

**成果物**: `sudo klav-daemon --config klav.toml` で起動し、キーボードでかなが打てる

### Phase 1: 日本語入力（v0.2） — 目標: 実用的な日本語ステノ

**ゴール**: StenoWordベースの理論で日本語のかな入力が実用レベルになる

- [ ] Layer 1 実装: ルールベース音節変換
  - 日本語 CV 構造（子音14種 × 母音5種）のマッピング
  - 濁音・半濁音・拗音・促音・長音の処理
- [ ] Layer 2 実装: 単語辞書
  - JSON 辞書フォーマット定義
  - 辞書データは完全自作（StenoWord理論のCV構造を参考に独自構築）
    - データソース: MeCab + UniDic 頻度情報から高頻度語を抽出
    - コード割当: StenoWordの子音母音ルールに基づきアルゴリズム生成
    - ライセンスクリーンな辞書を段階的に育てる
  - 起動時に HashMap へロード（O(1) 検索）
- [ ] 辞書の優先順位（ユーザー辞書 > ベース辞書）
- [ ] IME 連携（IBus / Fcitx5 経由で漢字変換）
- [ ] ストローク履歴バッファ（複数ストロークで1語を構成）

### Phase 2: 英語入力 + 多言語切替（v0.3） — 目標: 日英シームレス

**ゴール**: 英語ステノ理論を追加し、ストローク一発で日英を切り替えられる

- [ ] Theory プラグインシステム実装
  - 理論ごとに独立した TOML 定義 + 辞書セット
  - 実行時に理論を切替可能
- [ ] 英語ステノ理論の実装（Plover theory 互換）
  - Plover の JSON 辞書インポート
  - 英語ステノの基本ルール（初期子音・母音・末尾子音）
- [ ] 言語切替ストローク
- [ ] 切替時の出力モード自動変更（かな出力 ↔ アルファベット出力）

### Phase 3: GUI + ユーザビリティ（v0.4） — 目標: 一般ユーザーが使える

**ゴール**: 設定ウィンドウで主要な操作が完結する

- [ ] egui 設定ウィンドウ
  - 有効/無効トグル
  - キーマップ編集（ビジュアルエディタ）
  - 辞書管理（追加・削除・優先順位変更）
  - 言語・理論選択
- [ ] トレイアイコン（有効/無効・現在の言語表示）
- [ ] デーモン ↔ GUI の IPC（Unix socket）
- [ ] ユーザー辞書の動的追加（デーモン再起動不要）
- [ ] ストローク履歴表示（Paper Tape 相当）
- [ ] WPM / SPW メーター

### Phase 4: 漢字直接入力 + 独自理論（v0.5+） — 目標: 思考の速度

**ゴール**: IME 不要の漢字入力と、最適化された独自ステノ理論

- [ ] Layer 3 実装: 構造ベース漢字直接入力
  - 部首・画数ベースのコード体系設計
  - 常用漢字2,136字のコードマッピング
  - 同音異義語のストローク内解決
- [ ] 内蔵かな→漢字変換エンジン（libmozc FFI or 独自実装）
- [ ] 独自ステノ理論の設計・検証
  - 日本語音韻構造に最適化されたキー配置
  - 頻度分析に基づくコード割当
  - 人間工学を考慮した指の負荷分散
- [ ] 学習支援ツール（練習モード）

### Phase 5: クロスプラットフォーム（v1.0） — 目標: どこでも使える

**ゴール**: Linux / Windows / macOS で同等の体験

- [ ] Windows 対応
  - Raw Input API でキー取得
  - SendInput で文字出力
  - システムトレイ対応
- [ ] macOS 対応
  - IOKit HID でキー取得
  - CGEvent で文字出力
  - メニューバーアイコン対応
- [ ] クロスプラットフォーム抽象化レイヤー
  - `trait InputBackend` / `trait OutputBackend`
  - プラットフォーム固有コードを分離
- [ ] インストーラ / パッケージ
  - Linux: AppImage, deb, AUR
  - Windows: MSI
  - macOS: DMG

---

## 5. プロジェクト構造（Cargo Workspace）

```
klav/
├── Cargo.toml              # workspace 定義
├── ROADMAP.md              # この文書
├── README.md
├── LICENSE
│
├── klav-core/              # コアライブラリ（プラットフォーム非依存）
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── stroke.rs       # ストローク検出ロジック
│       ├── theory.rs       # 理論エンジン（プラグインインターフェース）
│       ├── dictionary.rs   # 辞書管理
│       ├── keymap.rs       # キーマップ定義・読み込み
│       ├── translator.rs   # ストローク → テキスト変換
│       └── config.rs       # TOML 設定読み込み
│
├── klav-daemon/            # デーモン（プラットフォーム固有）
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── input/
│       │   ├── mod.rs      # trait InputBackend
│       │   ├── evdev.rs    # Linux 実装
│       │   ├── rawinput.rs # Windows 実装（後回し）
│       │   └── iokit.rs    # macOS 実装（後回し）
│       ├── output/
│       │   ├── mod.rs      # trait OutputBackend
│       │   ├── uinput.rs   # Linux 実装
│       │   ├── sendinput.rs# Windows 実装（後回し）
│       │   └── cgevent.rs  # macOS 実装（後回し）
│       └── ipc.rs          # GUI との通信
│
├── klav-gui/               # 設定 GUI
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── app.rs          # egui アプリケーション
│
├── theories/               # ステノ理論定義
│   ├── ja-stenoword/
│   │   ├── theory.toml     # 理論定義
│   │   ├── rules.toml      # Layer 1: 音節ルール
│   │   └── dict_base.json  # Layer 2: 単語辞書
│   └── en-plover/
│       ├── theory.toml
│       └── dict_base.json
│
├── keymaps/                # キーマップ プリセット
│   ├── qwerty.toml
│   ├── ortho.toml          # 直交配列用
│   └── ergodox.toml        # ErgoDox 用
│
└── tools/                  # 補助ツール
    └── dict-gen/           # 辞書生成ツール（MeCab + UniDic → Klav辞書）
        ├── Cargo.toml
        └── src/
            └── main.rs
```

---

## 6. 理論設計メモ（Phase 1 向け）

### 6.1 日本語 Layer 1: 音節ルール

日本語の音節は CV（子音 + 母音）構造が基本。全ての清音は以下の組み合わせで表現可能:

```
子音（14種）: ∅, K, S, T, N, H, M, Y, R, W, G, Z, D, B, P
母音（5種）:  A, I, U, E, O
```

このうち基本子音（∅, K, S, T, N, H, M, Y, R, W）の10種は StenoWord と同じ発想でマッピングし、
濁音（G, Z, D, B）と半濁音（P）は修飾キーとの組み合わせで表現する。

```toml
# rules.toml の例
[syllable_rules]
# 清音: 子音キー + 母音キー
"" = { "A" = "あ", "I" = "い", "U" = "う", "E" = "え", "O" = "お" }
"K" = { "A" = "か", "I" = "き", "U" = "く", "E" = "け", "O" = "こ" }
"S" = { "A" = "さ", "I" = "し", "U" = "す", "E" = "せ", "O" = "そ" }

# 濁音: 濁音修飾 + 子音キー + 母音キー
[voiced_rules]
"K" = "G"   # か行 → が行
"S" = "Z"   # さ行 → ざ行
"T" = "D"   # た行 → だ行
"H" = "B"   # は行 → ば行

# 半濁音: 半濁音修飾 + 子音キー + 母音キー
[half_voiced_rules]
"H" = "P"   # は行 → ぱ行
```

### 6.2 英語 Theory（Phase 2 向け概要）

Plover theory 互換を基本とし、Plover の JSON 辞書をそのままインポート可能にする。
レイアウト:

```
#  #  #  #  #     #  #  #  #  #
S  T  P  H  *     F  P  L  T  D
S  K  W  R  *     R  B  G  S  Z
         A  O     E  U
```

---

## 7. マイルストーン

| Phase | バージョン | 目標 | 期間目安 |
|-------|-----------|------|---------|
| 0 | v0.1 | 動く最小構成（evdev→ストローク→uinput） | 2-3週間 |
| 1 | v0.2 | 実用的な日本語ステノ入力 | 1-2ヶ月 |
| 2 | v0.3 | 英語入力 + 多言語切替 | 1ヶ月 |
| 3 | v0.4 | GUI + ユーザビリティ | 1-2ヶ月 |
| 4 | v0.5+ | 漢字直接入力 + 独自理論 | 長期 |
| 5 | v1.0 | クロスプラットフォーム | 長期 |

---

## 8. ライセンス

**MIT / Apache-2.0 デュアルライセンス**（Rust エコシステム標準）。

- Plover のコードは一切流用しないため GPL 汚染なし
- 依存 crate（evdev, egui, tray-icon 等）はすべて Apache-2.0 / MIT
- 辞書データは完全自作のため、外部ライセンスの制約を受けない
- 商用利用・改変・再配布すべて自由

---

## 9. 参考資料

- [Plover (Open Steno Project)](https://github.com/openstenoproject/plover)
- [plover-japanese-stenoword](https://github.com/na2hiro/plover-japanese-stenoword)
- [StenoWord 学習資料](https://konomu.github.io/stenoword)
- [Plover Wiki - Steno layouts and supported languages](https://plover.wiki/index.php/Steno_layouts_and_supported_languages)
- [Plover - Designing Steno Systems](https://plover.readthedocs.io/en/latest/system_dev.html)
- [Frag's Japanese Theory (Plover Discord)](http://plover.stenoknight.com/2019/03/japanese-layout-plover-lessons-on-typey.html)
- [evdev crate (Rust)](https://github.com/emberian/evdev)
- [egui](https://github.com/emilk/egui)
- [tray-icon crate](https://lib.rs/crates/tray-icon)
