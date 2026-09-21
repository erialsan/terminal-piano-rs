//! 音符計算とキーマッピング（純粋関数集）
//!
//! shift: 基準音の半音オフセット。0=C(ド両端), 4=E(ミ両端)

/// キー文字 → 基準Cからの半音オフセット（基準C = MIDI 12*(octave+1)）
///
/// ',' と 'q' がどちらもオフセット12なのは意図的（VMPK と同じく低オクターブ行の折り返し）。
pub const KEYMAP: &[(char, i32)] = &[
    ('z', 0), ('s', 1), ('x', 2), ('d', 3), ('c', 4),
    ('v', 5), ('g', 6), ('b', 7), ('h', 8), ('n', 9), ('j', 10),
    ('m', 11), ('l', 13), (',', 12),
    ('q', 12), ('2', 13), ('w', 14), ('3', 15), ('e', 16),
    ('r', 17), ('5', 18), ('t', 19), ('6', 20), ('y', 21), ('7', 22),
    ('u', 23), ('i', 24), ('9', 25),
];

/// 高オクターブ段の白鍵: (キー, 半音オフセット)
pub const UPPER_WHITE: &[(char, i32)] = &[
    ('q', 12), ('w', 14), ('e', 16), ('r', 17), ('t', 19), ('y', 21), ('u', 23), ('i', 24),
];

/// 高オクターブ段の黒鍵: (キー, 直前の白鍵インデックス k)。発音は KEYMAP 側で引く。
pub const UPPER_BLACK: &[(char, u8)] = &[
    ('2', 0), ('3', 1), ('5', 3), ('6', 4), ('7', 5), ('9', 7),
];

/// 低オクターブ段の白鍵: (キー, 半音オフセット)
pub const LOWER_WHITE: &[(char, i32)] = &[
    ('z', 0), ('x', 2), ('c', 4), ('v', 5), ('b', 7), ('n', 9), ('m', 11), (',', 12),
];

/// 低オクターブ段の黒鍵: (キー, 直前の白鍵インデックス k)
pub const LOWER_BLACK: &[(char, u8)] = &[
    ('s', 0), ('d', 1), ('g', 3), ('h', 4), ('j', 5), ('l', 7),
];

const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// MIDI ノート番号 → 周波数 (Hz)。A4 (69) = 440.0
pub fn midi_to_freq(midi: i32) -> f32 {
    440.0 * 2f32.powf((midi - 69) as f32 / 12.0)
}

/// MIDI ノート番号 → 音名。例: 60 → "C4", 61 → "C#4"
pub fn note_name(midi: i32) -> String {
    let name = NOTE_NAMES[midi.rem_euclid(12) as usize];
    let octave = midi.div_euclid(12) - 1;
    format!("{name}{octave}")
}

/// キー文字・基準オクターブ・基準音シフト(半音)から MIDI ノート番号を計算。
/// キーが KEYMAP に無ければ None。
pub fn midi_for_key(key: char, octave: i32, shift: i32) -> Option<i32> {
    let offset = KEYMAP
        .iter()
        .find(|(c, _)| *c == key)
        .map(|(_, off)| *off)?;
    Some(12 * (octave + 1) + shift + offset)
}
