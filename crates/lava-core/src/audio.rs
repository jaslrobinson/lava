//! Real-time audio visualizer via parec (PipeWire/PulseAudio monitor capture).
//!
//! Uses a 4096-sample FFT window with 1024-sample hop (STFT) for 4x better
//! low-frequency resolution (~10.8 Hz/bin) while maintaining ~43 fps updates.

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use rustfft::{FftPlanner, num_complex::Complex};

pub const NUM_BANDS: usize = 24;
const SAMPLE_RATE: usize = 44100;

/// FFT window size -- 4096 gives ~10.8 Hz/bin, enough for distinct low-freq bands
const FFT_SIZE: usize = 4096;

/// Hop size -- read this many new samples per FFT, giving ~43 fps updates
const HOP_SIZE: usize = 1024;
const BYTES_PER_SAMPLE: usize = 2; // s16le mono

/// Shared audio band data (0.0-1.0 per band)
pub type SharedBands = Arc<Mutex<Vec<f32>>>;

pub fn new_shared_bands() -> SharedBands {
    Arc::new(Mutex::new(vec![0.0; NUM_BANDS]))
}

/// Start audio capture in a background thread.
/// Calls `on_bands` with smoothed band data and writes to temp file.
pub fn start_audio_capture<F>(on_bands: F, bands: SharedBands)
where
    F: Fn(&[f32]) + Send + 'static,
{
    thread::spawn(move || {
        run_capture_loop(on_bands, bands);
    });
}

fn run_capture_loop<F>(on_bands: F, bands: SharedBands)
where
    F: Fn(&[f32]) + Send + 'static,
{
    // Capture the default output monitor (what's playing through speakers).
    // On PipeWire with pipewire-pulse, @DEFAULT_MONITOR@ is the loopback source.
    let child_result = Command::new("parec")
        .args([
            "--format=s16le",
            "--rate=44100",
            "--channels=1",
            "--latency-msec=20",
            "--device=@DEFAULT_MONITOR@",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();

    let mut child = match child_result {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[audio] parec unavailable: {}. Visualizer disabled.", e);
            return;
        }
    };

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => return,
    };

    let mut reader = std::io::BufReader::with_capacity(HOP_SIZE * BYTES_PER_SAMPLE * 8, stdout);
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);

    // Precompute Hanning window coefficients
    let hanning: Vec<f32> = (0..FFT_SIZE)
        .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE - 1) as f32).cos()))
        .collect();

    // Ring buffer: always holds the last FFT_SIZE samples
    let mut ring: Vec<f32> = vec![0.0; FFT_SIZE];
    let mut ring_pos: usize = 0; // write position (oldest slot)

    let mut hop_buf = vec![0u8; HOP_SIZE * BYTES_PER_SAMPLE];
    let mut smoothed = vec![0.0f32; NUM_BANDS];

    loop {
        // Read one hop worth of new audio samples
        if reader.read_exact(&mut hop_buf).is_err() {
            break;
        }

        // Decode s16le and push into ring buffer
        for bytes in hop_buf.chunks_exact(2) {
            let s = i16::from_le_bytes([bytes[0], bytes[1]]) as f32 / 32768.0;
            ring[ring_pos] = s;
            ring_pos = (ring_pos + 1) % FFT_SIZE;
        }

        // Build windowed FFT input from ring buffer (oldest -> newest, with Hanning)
        let mut complex_buf: Vec<Complex<f32>> = (0..FFT_SIZE)
            .map(|i| {
                let sample = ring[(ring_pos + i) % FFT_SIZE];
                Complex::new(sample * hanning[i], 0.0)
            })
            .collect();

        fft.process(&mut complex_buf);

        let new_bands = compute_bands(&complex_buf, SAMPLE_RATE, NUM_BANDS);

        // Smooth: instant rise, exponential decay
        for i in 0..NUM_BANDS {
            smoothed[i] = if new_bands[i] > smoothed[i] {
                new_bands[i]
            } else {
                smoothed[i] * 0.78 + new_bands[i] * 0.22
            };
        }

        // Update shared state
        {
            let mut lock = bands.lock().unwrap();
            lock.copy_from_slice(&smoothed);
        }

        // Call the callback with band data
        on_bands(&smoothed);

        // Write to temp file for wallpaper WebKitGTK view
        if let Ok(json) = serde_json::to_string(&smoothed) {
            let path = std::env::temp_dir().join("lava-audio-bands.json");
            let tmp = std::env::temp_dir().join("lava-audio-bands.json.tmp");
            if std::fs::write(&tmp, &json).is_ok() {
                let _ = std::fs::rename(&tmp, &path);
            }
        }
    }

    let _ = child.kill();
    eprintln!("[audio] parec capture ended.");
}

/// Map FFT magnitude spectrum to NUM_BANDS logarithmic frequency bands.
/// With FFT_SIZE=4096 at 44100 Hz: ~10.8 Hz/bin -> distinct bands from ~40 Hz up.
fn compute_bands(fft_data: &[Complex<f32>], sample_rate: usize, num_bands: usize) -> Vec<f32> {
    let nyquist_bins = fft_data.len() / 2;
    let hz_per_bin = sample_rate as f32 / fft_data.len() as f32;

    // 40 Hz minimum: at 10.8 Hz/bin, bin 4 = 43 Hz -> all 24 bands are distinct
    let min_freq = 40.0f32;
    let max_freq = 16000.0f32;
    let log_range = (max_freq / min_freq).ln();

    let mut out = vec![0.0f32; num_bands];
    let mut prev_b1: usize = 0;

    for band_idx in 0..num_bands {
        let t0 = band_idx as f32 / num_bands as f32;
        let t1 = (band_idx + 1) as f32 / num_bands as f32;
        let f0 = min_freq * (t0 * log_range).exp();
        let f1 = min_freq * (t1 * log_range).exp();

        let b0 = ((f0 / hz_per_bin).floor() as usize).max(prev_b1.max(1));
        let b1 = ((f1 / hz_per_bin).ceil() as usize).clamp(b0 + 1, nyquist_bins);

        let count = (b1 - b0) as f32;
        let sum: f32 = fft_data[b0..b1].iter().map(|c| c.norm()).sum();

        // Normalize: divide by bin count, FFT size, and apply perceptual gain.
        // Music energy drops ~6 dB/octave, so higher bands need exponentially
        // more boost to appear equally active in a visualizer.
        let t = band_idx as f32 / num_bands as f32;
        let perceptual_gain = (1.0 + t * 3.0).powi(2); // 1x at low -> 16x at high
        out[band_idx] = (sum / count / fft_data.len() as f32 * 20.0 * perceptual_gain).min(1.0);

        prev_b1 = b1;
    }

    out
}
