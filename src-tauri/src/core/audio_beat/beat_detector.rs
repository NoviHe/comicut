use anyhow::Result;
use rustfft::{num_complex::Complex, FftPlanner};
use std::path::Path;

use crate::models::audio::{BeatMap, BeatMarker};

const WIN: usize = 2048;
const HOP: usize = 512;

/// Onset flux + low-band energy per frame (hop = HOP samples).
/// Flux is dominated by the 30-180Hz kick/sub band; a light high-band term catches snares/cowbells.
fn spectral_features(samples: &[f32], sr: u32) -> (Vec<f32>, Vec<f32>) {
    let bin_hz = sr as f32 / WIN as f32;
    let lo_a = ((30.0 / bin_hz) as usize).max(1);
    let lo_b = ((180.0 / bin_hz).ceil() as usize).max(lo_a + 1);
    let hi_b = ((4000.0 / bin_hz) as usize).min(WIN / 2 - 1).max(lo_b + 1);

    let hann: Vec<f32> = (0..WIN)
        .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / WIN as f32).cos()))
        .collect();
    let fft = FftPlanner::<f32>::new().plan_fft_forward(WIN);

    let mut prev = vec![0.0f32; hi_b + 1];
    let (mut flux, mut low) = (Vec::new(), Vec::new());
    let mut start = 0usize;
    while start + WIN <= samples.len() {
        let mut buf: Vec<Complex<f32>> = samples[start..start + WIN]
            .iter()
            .zip(&hann)
            .map(|(s, w)| Complex::new(s * w, 0.0))
            .collect();
        fft.process(&mut buf);

        let (mut f_low, mut f_high, mut e_low) = (0.0f32, 0.0f32, 0.0f32);
        for k in lo_a..=hi_b {
            let mag = buf[k].norm() / 512.0;
            let lg = (1.0 + 20.0 * mag).ln();
            let d = (lg - prev[k]).max(0.0);
            prev[k] = lg;
            if k <= lo_b {
                f_low += d;
                e_low += mag;
            } else {
                f_high += d;
            }
        }
        let n_low = (lo_b - lo_a + 1) as f32;
        let n_high = (hi_b - lo_b) as f32;
        flux.push(f_low / n_low + 0.25 * f_high / n_high);
        low.push(e_low);
        start += HOP;
    }
    (flux, low)
}

fn prefix_sums(v: &[f32]) -> Vec<f32> {
    let mut out = Vec::with_capacity(v.len() + 1);
    out.push(0.0);
    for x in v {
        out.push(out.last().unwrap() + x);
    }
    out
}

fn mean(p: &[f32], a: usize, b: usize) -> f32 {
    let (a, b) = (a.min(p.len() - 1), b.min(p.len() - 1));
    if b <= a { 0.0 } else { (p[b] - p[a]) / (b - a) as f32 }
}

/// Montagem/Phonk beat map: onset envelope -> tempo (autocorrelation) -> beat grid phase -> grid snapped
/// to real kick peaks. Main drop = biggest rise in low-band energy.
pub fn detect_montagem_beats(audio_path: &str, samples: &[f32], sample_rate: u32) -> Result<BeatMap> {
    let sr = sample_rate.max(1) as f32;
    let duration_sec = samples.len() as f32 / sr;
    let dt = HOP as f32 / sr;
    let (flux, low) = spectral_features(samples, sample_rate);
    let n = flux.len();

    // Onset envelope: remove local mean (+-0.4s), scale by 95th percentile.
    let pf = prefix_sums(&flux);
    let w = ((0.4 / dt) as usize).max(2);
    let mut env: Vec<f32> = (0..n)
        .map(|i| (flux[i] - mean(&pf, i.saturating_sub(w), i + w + 1)).max(0.0))
        .collect();
    let mut sorted = env.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p95 = sorted.get(n * 95 / 100).copied().unwrap_or(1.0).max(1e-6);
    env.iter_mut().for_each(|e| *e /= p95);

    // Tempo: autocorrelation over 70-200 BPM, log-gaussian prior around 135 BPM (octave tie-break).
    let lag_min = ((60.0 / 200.0 / dt) as usize).max(2);
    let lag_max = ((60.0 / 70.0 / dt) as usize).min(n / 2);
    let mut period = 60.0 / 135.0 / dt;
    if lag_min + 2 < lag_max {
        let ac: Vec<f32> = (lag_min - 1..=lag_max + 1)
            .map(|lag| {
                let bpm = 60.0 / (lag as f32 * dt);
                let prior = (-0.5 * ((bpm / 135.0).log2() / 0.6).powi(2)).exp();
                let s: f32 = (0..n - lag).map(|i| env[i] * env[i + lag]).sum();
                prior * s / (n - lag) as f32
            })
            .collect();
        let best = (1..ac.len() - 1)
            .max_by(|&a, &b| ac[a].partial_cmp(&ac[b]).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(1);
        let (y0, y1, y2) = (ac[best - 1], ac[best], ac[best + 1]);
        let denom = y0 - 2.0 * y1 + y2;
        let frac = if denom.abs() > 1e-9 { 0.5 * (y0 - y2) / denom } else { 0.0 };
        period = (lag_min - 1 + best) as f32 + frac.clamp(-1.0, 1.0);
    }

    // Phase: grid offset that lands on the most onset energy.
    let at = |t: f32| -> f32 {
        let c = t.round() as isize;
        (c - 1..=c + 1)
            .filter(|&i| i >= 0 && (i as usize) < n)
            .map(|i| env[i as usize])
            .fold(0.0, f32::max)
    };
    let mut best_phase = 0.0f32;
    let mut best_score = -1.0f32;
    for ph in 0..period.ceil() as usize {
        let mut score = 0.0;
        let mut t = ph as f32;
        while (t as usize) < n {
            score += at(t);
            t += period;
        }
        if score > best_score {
            best_score = score;
            best_phase = ph as f32;
        }
    }

    // Grid points snapped to the strongest onset within +-15% of a period (re-syncs tempo drift).
    let radius = ((0.15 * period).round() as isize).max(1);
    let mut beats: Vec<BeatMarker> = Vec::new();
    let mut k = 0.0f32;
    loop {
        let t = best_phase + k * period;
        if t as usize >= n {
            break;
        }
        k += 1.0;
        let c = t.round() as isize;
        let (mut pi, mut pv) = (c.max(0) as usize, 0.0f32);
        for i in (c - radius).max(0)..=(c + radius).min(n as isize - 1) {
            if env[i as usize] > pv {
                pv = env[i as usize];
                pi = i as usize;
            }
        }
        let (time, intensity) = if pv >= 0.15 {
            (pi as f32 * dt, pv.min(1.0))
        } else {
            (t * dt, 0.1)
        };
        if beats.last().is_some_and(|b| time - b.timestamp_sec < 0.6 * period * dt) {
            continue;
        }
        beats.push(BeatMarker {
            timestamp_sec: time,
            intensity,
            is_bass_drop: false,
            is_sub_bass_hit: intensity >= 0.6,
        });
    }
    // Skip silent intro: start at the first audible beat.
    if let Some(first) = beats.iter().position(|b| b.intensity >= 0.15) {
        beats.drain(..first);
    }

    // Main drop: largest rise of low-band energy (2s after vs 2s before), snapped to the nearest beat.
    let pl = prefix_sums(&low);
    let dw = (2.0 / dt) as usize;
    let mut drop_t: Option<f32> = None;
    if n > 2 * dw + 2 {
        let (mut best, mut best_i) = (f32::MIN, 0usize);
        for i in dw.max((1.0 / dt) as usize)..n - dw {
            let s = mean(&pl, i, i + dw) - mean(&pl, i - dw, i);
            if s > best {
                best = s;
                best_i = i;
            }
        }
        if best > 0.15 * mean(&pl, 0, n) {
            drop_t = Some(best_i as f32 * dt);
        }
    }
    let drop_idx = match drop_t {
        Some(t) => beats
            .iter()
            .enumerate()
            .min_by(|a, b| {
                (a.1.timestamp_sec - t).abs().partial_cmp(&(b.1.timestamp_sec - t).abs()).unwrap()
            })
            .map(|(i, _)| i),
        // No clear build-up: strongest beat after 1.2s.
        None => beats
            .iter()
            .enumerate()
            .filter(|(_, b)| b.timestamp_sec >= 1.2)
            .max_by(|a, b| a.1.intensity.partial_cmp(&b.1.intensity).unwrap())
            .map(|(i, _)| i),
    };
    let mut main_drop_sec = (duration_sec * 0.35).min(3.6);
    if let Some(i) = drop_idx {
        beats[i].is_bass_drop = true;
        main_drop_sec = beats[i].timestamp_sec;
    }

    // Waveform for the UI: 96 RMS bars.
    let bars = 96usize;
    let chunk = (samples.len() / bars).max(1);
    let mut peaks: Vec<f32> = samples
        .chunks(chunk)
        .take(bars)
        .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
        .collect();
    let pmax = peaks.iter().copied().fold(1e-6, f32::max);
    peaks.iter_mut().for_each(|p| *p = (*p / pmax).max(0.04));

    let file_name = Path::new(audio_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "montagem_track.mp3".to_string());

    println!(
        "[beat] bpm={:.1} beats={} drop={:.2}s dur={:.1}s",
        60.0 / (period * dt),
        beats.len(),
        main_drop_sec,
        duration_sec
    );

    Ok(BeatMap {
        audio_path: audio_path.to_string(),
        file_name,
        duration_sec,
        bpm: (600.0 / (period * dt)).round() / 10.0,
        main_drop_sec,
        beats,
        waveform_peaks: peaks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 120 BPM kick train (60Hz decaying bursts) starting at 0.25s, 12s long.
    #[test]
    fn finds_tempo_and_phase() {
        let sr = 44100u32;
        let mut s = vec![0.0f32; sr as usize * 12];
        for k in 0..24 {
            let start = (0.25 + 0.5 * k as f32) * sr as f32;
            for j in 0..(0.15 * sr as f32) as usize {
                let t = j as f32 / sr as f32;
                s[start as usize + j] += (-t * 30.0).exp() * (2.0 * std::f32::consts::PI * 60.0 * t).sin();
            }
        }
        let m = detect_montagem_beats("x.wav", &s, sr).unwrap();
        assert!((m.bpm - 120.0).abs() < 2.0, "bpm {}", m.bpm);
        let first = m.beats[0].timestamp_sec;
        assert!((first - 0.25).abs() < 0.05, "first beat {}", first);
    }
}
