//! TUI バーチャルピアノ（VMPK風）

mod audio;
mod piano;
mod ui;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crossterm::event::{
    self, Event, KeyCode, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};

use audio::{AudioEngine, NoteHandle};

/// 押下反転・フォールバック音の最短相当の表示時間（非対応端末の自動劣化用）
const FLASH: Duration = Duration::from_millis(150);

pub struct AppState {
    /// 基準オクターブ（クランプ 1..=6）
    pub octave: i32,
    /// 反転表示用: キー → 押下時刻
    pub pressed: HashMap<char, Instant>,
    /// 長押し中のノート（kitty 拡張対応端末のみ）
    held: HashMap<char, NoteHandle>,
    /// 直近の発音 MIDI 番号
    pub last_note: Option<u8>,
    /// None なら無音モード
    pub audio: Option<AudioEngine>,
    /// Release/Repeat イベントを取得できるか
    enhanced: bool,
}

impl AppState {
    fn new(enhanced: bool) -> Self {
        Self {
            octave: 4,
            pressed: HashMap::new(),
            held: HashMap::new(),
            last_note: None,
            audio: AudioEngine::new(),
            enhanced,
        }
    }
}

fn main() -> std::io::Result<()> {
    // 先に端末を初期化（ghostty 実測: raw モード等が有効になった後でないと
    // Kitty フラグの Push が端末側で適用されない）
    let mut terminal = ratatui::init();

    // Kitty キーボードプロトコル: Press/Release/Repeat を区別可能に。
    // Release を得るには DISAMBIGUATE_ESCAPE_CODES との併用が必須
    // （非対応端末では失敗→フォールバック）
    let enhanced = crossterm::execute!(
        std::io::stdout(),
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES,
        )
    )
    .is_ok();

    // 端末がフラグを適用するまでのラグ対策（ghostty 実測で適用遅延あり）:
    // 少し待って、端末からの応答イベントがあれば捨てる
    let settle_until = Instant::now() + Duration::from_millis(300);
    while Instant::now() < settle_until {
        match event::poll(Duration::from_millis(30)) {
            Ok(true) => {
                let _ = event::read();
            }
            _ => break,
        }
    }

    let mut state = AppState::new(enhanced);
    let result = run_loop(&mut terminal, &mut state);

    if enhanced {
        let _ = crossterm::execute!(std::io::stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    result
}

fn run_loop(
    terminal: &mut ratatui::DefaultTerminal,
    state: &mut AppState,
) -> std::io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, state))?;

            // 非拡張端末では Release が来ないため、FLASH 時間で反転を消す（発音は 2 秒固定）
            if !state.enhanced {
                state.pressed.retain(|_, t| t.elapsed() < FLASH);
            }
            // 安全網: Release が来ない端末（Push 成功≠プロトコル解釈）での鳴りっぱなし防止。
            // 実際の長押しで 60 秒超は到達しない前提
            let stuck: Vec<char> = state
                .pressed
                .iter()
                .filter(|(_, t)| t.elapsed() > Duration::from_secs(60))
                .map(|(c, _)| *c)
                .collect();
            for c in stuck {
                if let Some(h) = state.held.remove(&c) {
                    h.release();
                }
                state.pressed.remove(&c);
            }

            if event::poll(Duration::from_millis(30))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        // 終了は Press のみ（Repeat/Release で二重回避）
                        KeyCode::Esc if key.kind == KeyEventKind::Press => break,
                        KeyCode::Char('c')
                            if key.modifiers.contains(KeyModifiers::CONTROL)
                                && key.kind == KeyEventKind::Press =>
                        {
                            break
                        }
                        KeyCode::Right if key.kind != KeyEventKind::Release => {
                            state.octave = (state.octave + 1).min(6)
                        }
                        KeyCode::Left if key.kind != KeyEventKind::Release => {
                            state.octave = (state.octave - 1).max(1)
                        }
                        KeyCode::Char(c) => {
                            let c = c.to_ascii_lowercase();
                            match (state.enhanced, key.kind) {
                                // 長押し: リリースでフェード停止
                                (true, KeyEventKind::Release) => {
                                    if let Some(h) = state.held.remove(&c) {
                                        h.release();
                                    }
                                    state.pressed.remove(&c);
                                }
                                // リピートは無視（押し続けている間は鳴り続ける）
                                (true, KeyEventKind::Repeat) => {}
                                // Press: 長押し開始 or 固定 2 秒音（フォールバック）
                                (_, KeyEventKind::Press)
                                | (_, KeyEventKind::Release)
                                | (_, KeyEventKind::Repeat) => {
                                    let enhanced_press =
                                        state.enhanced && key.kind == KeyEventKind::Press;
                                    if enhanced_press && state.held.contains_key(&c) {
                                        // 重複 Press（念のため）
                                        continue;
                                    }
                                    if let Some(midi) = piano::midi_for_key(c, state.octave) {
                                        let midi = midi as u8; // 上限: 12*7 + 25 = 109 < 128、安全
                                        if let Some(audio) = state.audio.as_mut() {
                                            if enhanced_press {
                                                let h = audio.note_on_hold(midi);
                                                state.held.insert(c, h);
                                            } else if !state.enhanced {
                                                audio.note_on(midi);
                                            }
                                        }
                                        state.pressed.insert(c, Instant::now());
                                        state.last_note = Some(midi);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
    }
    Ok(())
}
