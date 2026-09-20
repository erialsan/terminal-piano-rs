//! 音声エンジン（rodio 0.22）

use rodio::{buffer::SamplesBuffer, mixer::Mixer, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use std::collections::HashMap;
use std::num::{NonZeroU16, NonZeroU32};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use crate::piano;

pub const SAMPLE_RATE: u32 = 44_100;

/// ノートの長さ（秒）
const DUR: f32 = 2.0;
/// アタック（秒）。線形 0→1
const ATTACK: f32 = 0.01;
/// リリース（秒）。末尾を線形に 0 へ畳みポップ音を防ぐ
const RELEASE: f32 = 0.03;
/// 減衰係数（exp(-DECAY * t_after_attack)）
const DECAY: f32 = 3.0;
/// 全体ゲイン
const GAIN: f32 = 0.25;
/// 倍音合成の正規化係数（1.0 + 0.35 + 0.12）
const NORM: f32 = 1.47;

pub struct AudioEngine {
    /// 生存維持のため保持（drop で再生停止）
    _sink: MixerDeviceSink,
    mixer: Mixer,
    /// MIDI 番号 → 合成済み PCM
    cache: HashMap<u8, Vec<f32>>,
}

impl AudioEngine {
    /// デフォルト出力デバイスを開く。失敗時は None（無音モードで起動続行）
    pub fn new() -> Option<Self> {
        let sink = DeviceSinkBuilder::open_default_sink().ok()?;
        let mixer = sink.mixer().clone();
        Some(Self {
            _sink: sink,
            mixer,
            cache: HashMap::new(),
        })
    }

    /// ノートを発音。初回は synthesize で生成し cache に格納、以降は clone。
    /// 発火は fire-and-forget（各音は 2 秒で自動消滅）。
    pub fn note_on(&mut self, midi: u8) {
        let samples = self
            .cache
            .entry(midi)
            .or_insert_with(|| synthesize(midi))
            .clone();
        let buf = SamplesBuffer::new(
            NonZeroU16::new(1).unwrap(),
            NonZeroU32::new(SAMPLE_RATE).unwrap(),
            samples,
        );
        self.mixer.add(buf);
    }

    /// 長押し用ノートを開始。リリースは返した NoteHandle::release で。
    pub fn note_on_hold(&mut self, midi: u8) -> NoteHandle {
        let player = Player::connect_new(&self.mixer);
        let release_flag = Arc::new(AtomicBool::new(false));
        player.append(SustainSource::new(midi, release_flag.clone()));
        player.play();
        NoteHandle { player, release_flag }
    }
}

/// 長押し中のノートのハンドル。
/// release はフラグを立てて detach するだけ —— フェードは音源側で
/// サンプル単位のリニアランプとして描画する（ジッパーノイズ/クリック音防止）。
pub struct NoteHandle {
    player: Player,
    release_flag: Arc<AtomicBool>,
}

impl NoteHandle {
    pub fn release(self) {
        self.release_flag.store(true, Ordering::SeqCst);
        // detach で Player ドロップによる即時停止を防ぎ、
        // ソースがフェード完了（None を返す）まで再生を継続させる
        self.player.detach();
    }
}

/// リリースフェード時間（秒）。ピアノの弦の余韻っぽい速すぎない減衰
const RELEASE_FADE: f32 = 0.15;

/// 長押し持続音源。アタック後は一定音量で鳴り続け、
/// リリースフラグが立つと RELEASE_FADE 秒かけて 0 へ線形減衰して終了する。
struct SustainSource {
    midi: u8,
    t: u64,
    release_flag: Arc<AtomicBool>,
    /// リリース開始サンプル位置
    released_at: Option<u64>,
}

impl SustainSource {
    fn new(midi: u8, release_flag: Arc<AtomicBool>) -> Self {
        Self {
            midi,
            t: 0,
            release_flag,
            released_at: None,
        }
    }
}

impl Iterator for SustainSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let release_samples = (RELEASE_FADE * SAMPLE_RATE as f32) as u64;
        let rel_gain = match self.released_at {
            Some(r0) => {
                let elapsed = self.t.saturating_sub(r0);
                if elapsed >= release_samples {
                    return None; // フェード完了 → ソース終了（Player 側で回収される）
                }
                1.0 - elapsed as f32 / release_samples as f32
            }
            None => {
                if self.release_flag.load(Ordering::SeqCst) {
                    self.released_at = Some(self.t);
                    1.0
                } else {
                    1.0
                }
            }
        };

        let t = self.t as f32 / SAMPLE_RATE as f32;
        self.t += 1;
        let w = piano::midi_to_freq(self.midi as i32) * t;
        let tone = ((w * std::f32::consts::TAU).sin()
            + 0.35 * (2.0 * w * std::f32::consts::TAU).sin()
            + 0.12 * (3.0 * w * std::f32::consts::TAU).sin())
            / NORM;
        let env = if t < ATTACK { t / ATTACK } else { 1.0 };
        Some(tone * env * rel_gain * GAIN)
    }
}

impl Source for SustainSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> NonZeroU16 {
        NonZeroU16::new(1).unwrap()
    }
    fn sample_rate(&self) -> NonZeroU32 {
        NonZeroU32::new(SAMPLE_RATE).unwrap()
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// ピアノ風エンベロープ付きサンプル列を生成（MIDI → 44.1kHz mono f32）
fn synthesize(midi: u8) -> Vec<f32> {
    let freq = piano::midi_to_freq(midi as i32);
    let n = (DUR * SAMPLE_RATE as f32) as usize;
    let release_start = DUR - RELEASE;
    (0..n)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let w = freq * t;
            let tone =
                ((w * std::f32::consts::TAU).sin()
                    + 0.35 * (2.0 * w * std::f32::consts::TAU).sin()
                    + 0.12 * (3.0 * w * std::f32::consts::TAU).sin())
                    / NORM;
            let env = if t < ATTACK {
                t / ATTACK
            } else {
                (-DECAY * (t - ATTACK)).exp()
            };
            let rel = if t > release_start {
                (DUR - t) / RELEASE
            } else {
                1.0
            };
            tone * env * rel * GAIN
        })
        .collect()
}
