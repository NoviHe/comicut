use anyhow::Result;
use rustfft::{num_complex::Complex, FftPlanner};
use std::path::Path;

use crate::models::audio::{BeatMap, BeatMarker};

/// Menganalisis frekuensi Sub-Bass & Kick (40Hz - 150Hz) khas lagu Montagem / Phonk
/// untuk menemukan ketukan beat dan titik Main Bass Drop.
pub fn detect_montagem_beats(
    audio_path: &str,
    samples: &[f32],
    sample_rate: u32,
) -> Result<BeatMap> {
    let duration_sec = samples.len() as f32 / sample_rate.max(1) as f32;
    let window_size = 2048usize;
    let hop_size = 1024usize;

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(window_size);

    let bin_hz = sample_rate as f32 / window_size as f32;
    let min_bass_bin = ((40.0 / bin_hz).floor() as usize).max(1);
    let max_bass_bin = ((150.0 / bin_hz).ceil() as usize).min(window_size / 2);

    let mut bass_energies: Vec<(f32, f32)> = Vec::new();
    let mut idx = 0usize;

    while idx + window_size <= samples.len() {
        let mut buffer: Vec<Complex<f32>> = samples[idx..idx + window_size]
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                let hann = 0.5
                    * (1.0
                        - (2.0 * std::f32::consts::PI * i as f32 / window_size as f32).cos());
                Complex::new(s * hann, 0.0)
            })
            .collect();

        fft.process(&mut buffer);

        let mut bass_sum = 0.0f32;
        for bin in min_bass_bin..=max_bass_bin {
            bass_sum += buffer[bin].norm();
        }
        let time_sec = idx as f32 / sample_rate as f32;
        bass_energies.push((time_sec, bass_sum));

        idx += hop_size;
    }

    let max_energy = bass_energies
        .iter()
        .map(|(_, e)| *e)
        .fold(0.0001f32, f32::max);

    for item in bass_energies.iter_mut() {
        item.1 = (item.1 / max_energy).clamp(0.0, 1.0);
    }

    // Cari local peaks (onset bass kick) dengan jarak minimal 0.32 detik (~185 BPM maks)
    let mut beats: Vec<BeatMarker> = Vec::new();
    let mut last_beat_time = -1.0f32;

    for i in 2..bass_energies.len().saturating_sub(2) {
        let (t, e) = bass_energies[i];
        let prev = bass_energies[i - 1].1;
        let next = bass_energies[i + 1].1;

        if e > 0.48 && e >= prev && e >= next && (t - last_beat_time) >= 0.35 {
            beats.push(BeatMarker {
                timestamp_sec: t,
                intensity: e,
                is_bass_drop: false,
                is_sub_bass_hit: e >= 0.68,
            });
            last_beat_time = t;
        }
    }

    // Tentukan Main Bass Drop (ketukan dengan lonjakan intensitas terbesar setelah 1.5 detik)
    let mut main_drop_sec = (duration_sec * 0.35).min(3.6);
    let mut best_drop_score = 0.0f32;
    let mut drop_idx_opt = None;

    for (i, b) in beats.iter().enumerate() {
        if b.timestamp_sec >= 1.2 && b.intensity > best_drop_score {
            best_drop_score = b.intensity;
            main_drop_sec = b.timestamp_sec;
            drop_idx_opt = Some(i);
        }
    }
    if let Some(di) = drop_idx_opt {
        beats[di].is_bass_drop = true;
    }

    // Estimasi BPM
    let bpm = if beats.len() >= 2 {
        let avg_interval =
            (beats.last().unwrap().timestamp_sec - beats.first().unwrap().timestamp_sec)
                / (beats.len().saturating_sub(1) as f32);
        if avg_interval > 0.1 {
            (60.0 / avg_interval).clamp(90.0, 180.0).round()
        } else {
            135.0
        }
    } else {
        135.0
    };

    // Ringkas 48 bar waveform peaks untuk UI
    let num_bars = 48usize;
    let chunk = (bass_energies.len() / num_bars).max(1);
    let waveform_peaks: Vec<f32> = bass_energies
        .chunks(chunk)
        .take(num_bars)
        .map(|c| c.iter().map(|x| x.1).fold(0.15f32, f32::max))
        .collect();

    let file_name = Path::new(audio_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "montagem_track.mp3".to_string());

    Ok(BeatMap {
        audio_path: audio_path.to_string(),
        file_name,
        duration_sec,
        bpm,
        main_drop_sec,
        beats,
        waveform_peaks,
    })
}
