//! 描画（VMPK風2段鍵盤、画面全体使用）
//!
//! 白鍵は縦線 │ で区切った多段セル、黒鍵は █ の塊を白鍵境界へ被せる。
//! 押下中は REVERSED|BOLD。幅・高さは端末サイズから動的に計算する。

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

/// 端末テーマ追従のパレット。名前付き ANSI 色（パレット 0–15）は
/// ghostty などのテーマ設定に自動的に合わせられる
mod theme {
    use ratatui::style::Color;
    /// 補助的な情報（区切り線・音名など）
    pub const DIM: Color = Color::DarkGray;
    /// 黒鍵（テーマの Bright Black）
    pub const BLACK_KEY: Color = Color::DarkGray;
    /// 数値などのアクセント
    pub const ACCENT: Color = Color::Cyan;
    /// 正常ステータス
    pub const OK: Color = Color::Green;
    /// 異常ステータス
    pub const WARN: Color = Color::Red;
}

use crate::{piano, AppState};

/// 白鍵セル幅の下限
const MIN_CELL: usize = 4;
/// 1段あたりの白鍵数
const WHITE_KEYS: usize = 8;

/// 行バッファ上のスタイル区別
#[derive(Clone, Copy, PartialEq, Eq)]
enum Paint {
    Normal,
    Dim,
    WhitePressed,
    Black,
    BlackPressed,
}

fn style_of(p: Paint) -> Style {
    match p {
        Paint::Normal => Style::default(),
        Paint::Dim => Style::default().fg(theme::DIM),
        Paint::Black => Style::default().fg(theme::BLACK_KEY),
        Paint::WhitePressed => Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD),
        Paint::BlackPressed => {
            Style::default()
                .fg(theme::BLACK_KEY)
                .add_modifier(Modifier::REVERSED | Modifier::BOLD)
        }
    }
}

fn is_pressed(state: &AppState, key: char) -> bool {
    state.pressed.contains_key(&key)
}

/// ratatui の行をスタイル区間ごとの Span に変換
fn spans_from_row(chars: &[char], paints: &[Paint]) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let mut j = i + 1;
        while j < chars.len() && paints[j] == paints[i] {
            j += 1;
        }
        let text: String = chars[i..j].iter().collect();
        let style = style_of(paints[i]);
        if style == Style::default() {
            spans.push(Span::raw(text));
        } else {
            spans.push(Span::styled(text, style));
        }
        i = j;
    }
    Line::from(spans)
}

/// 1オクターブ分の鍵盤セクション（高さ h 行、白鍵セル幅 w）
fn key_section(
    state: &AppState,
    black: &[(char, u8)],
    white: &[(char, i32)],
    octave: i32,
    shift: i32,
    h: usize,
    w: usize,
    width: usize,
) -> Vec<Line<'static>> {
    let cols = (WHITE_KEYS * w).min(width);
    // 黒鍵領域の高さ: 全体の 3/5（最低1、白鍵専用エリアを最低2行残す）
    let black_h = (h * 3 / 5).clamp(1, h.saturating_sub(2).max(1));
    let label_row = (black_h + 1).min(h - 1); // 白鍵ラベル行
    let name_row = h - 1; // 音名行
    let cap_w = (w * 3 / 5).max(3).min(w); // 黒鍵幅 = 白鍵の約60%
    let base = 12 * (octave + 1) + shift;

    (0..h)
        .map(|r| {
            let mut chars = vec![' '; cols];
            let mut paints = vec![Paint::Normal; cols];

            // 白鍵: 押下中セルの内側を塗る
            for (i, (key, _)) in white.iter().enumerate() {
                if is_pressed(state, *key) {
                    for c in i * w..((i + 1) * w).min(cols) {
                        paints[c] = Paint::WhitePressed;
                    }
                }
            }

            // 白鍵境界の縦線
            for i in 1..WHITE_KEYS {
                if i * w < cols {
                    chars[i * w] = '│';
                    if paints[i * w] == Paint::Normal {
                        paints[i * w] = Paint::Dim;
                    }
                }
            }

            // 黒鍵キャップ（黒鍵領域のみ、白鍵境界へ中央揃え）
            if r < black_h {
                for (key, k) in black {
                    let boundary = (*k as usize + 1) * w;
                    let x = boundary
                        .saturating_sub(cap_w / 2 + 1)
                        .min(cols.saturating_sub(cap_w));
                    let paint = if is_pressed(state, *key) {
                        Paint::BlackPressed
                    } else {
                        Paint::Black
                    };
                    for c in x..x + cap_w {
                        if c < cols {
                            chars[c] = '█';
                            paints[c] = paint;
                        }
                    }
                    // キー名をキャップ下端の行に
                    if r == black_h - 1 {
                        chars[x + cap_w / 2] = *key;
                    }
                }
            }

            // 白鍵のキーラベル
            if r == label_row {
                for (i, (key, _)) in white.iter().enumerate() {
                    let c = i * w + w / 2;
                    if c < cols {
                        chars[c] = *key;
                    }
                }
            }

            // 音名（最下行）
            if r == name_row {
                for (i, (_, off)) in white.iter().enumerate() {
                    let name = piano::note_name(base + off);
                    let name_chars: Vec<char> = name.chars().collect();
                    let start = i * w + w.saturating_sub(name_chars.len()) / 2;
                    for (j, ch) in name_chars.iter().enumerate() {
                        let c = start + j;
                        if c < cols {
                            chars[c] = *ch;
                            if paints[c] == Paint::Normal {
                                paints[c] = Paint::Dim;
                            }
                        }
                    }
                }
            }

            spans_from_row(&chars, &paints)
        })
        .collect()
}

pub fn draw(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let block = Block::bordered().title(" TUI Piano ");
    let inner = block.inner(area);
    let width = inner.width as usize;
    let w = (width / WHITE_KEYS).max(MIN_CELL);

    let dim = Style::default().fg(theme::DIM);
    let accent = Style::default()
        .fg(theme::ACCENT)
        .add_modifier(Modifier::BOLD);
    let sound = if state.audio.is_some() { "OK" } else { "無し" };
    let sound_style = Style::default().fg(if state.audio.is_some() {
        theme::OK
    } else {
        theme::WARN
    });
    let last_spans = match state.last_note {
        Some(m) => vec![
            Span::styled(piano::note_name(m as i32), accent),
            Span::styled(
                format!(" {:.1}Hz", piano::midi_to_freq(m as i32)),
                dim,
            ),
        ],
        None => vec![Span::styled("----", dim)],
    };
    let mut header_spans = vec![
        Span::raw("Octave: "),
        Span::styled(state.octave.to_string(), accent),
        Span::styled(
            format!(
                " ({}–{})",
                piano::note_name(12 * (state.octave + 1) + state.key_shift),
                piano::note_name(12 * (state.octave + 1) + state.key_shift + 24)
            ),
            dim,
        ),
        Span::raw("   基準: "),
        Span::styled(if state.key_shift == 0 { "C" } else { "E" }, accent),
        Span::raw("   Last: "),
    ];
    header_spans.extend(last_spans);
    header_spans.push(Span::raw("   音源: "));
    header_spans.push(Span::styled(sound, sound_style));
    let header = Line::from(header_spans);
    let footer = Line::from(Span::styled(
        "Q–I=高 Z–,=低  ←/→ オクターブ  Tab 基準C⇔E  Esc 終了",
        dim,
    ));

    // 上下段の高さ: 固定5行（ヘッダ/フッタ/3つの隙間）を除いた残りを半分ずつ
    let sh = ((inner.height as usize).saturating_sub(5) / 2).max(3);

    let mut lines: Vec<Line<'static>> = Vec::with_capacity(inner.height as usize);
    lines.push(header);
    lines.push(Line::default());
    lines.extend(key_section(
        state,
        piano::UPPER_BLACK,
        piano::UPPER_WHITE,
        state.octave,
        state.key_shift,
        sh,
        w,
        width,
    ));
    lines.push(Line::default());
    lines.extend(key_section(
        state,
        piano::LOWER_BLACK,
        piano::LOWER_WHITE,
        state.octave,
        state.key_shift,
        sh,
        w,
        width,
    ));
    lines.push(Line::default());
    lines.push(footer);

    frame.render_widget(Paragraph::new(lines).block(block), area);
}
