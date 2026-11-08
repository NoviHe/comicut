use anyhow::{anyhow, Result};
use std::fs::File;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct DecodedMonoAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

pub fn decode_audio_mono(path: &str) -> Result<DecodedMonoAudio> {
    let file = File::open(path).map_err(|e| anyhow!("Gagal membuka file audio {}: {}", path, e))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if path.to_lowercase().ends_with(".mp3") {
        hint.with_extension("mp3");
    } else if path.to_lowercase().ends_with(".wav") {
        hint.with_extension("wav");
    }

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| anyhow!("Tidak ditemukan track audio di dalam file"))?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2).max(1);

    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    let mut mono_samples: Vec<f32> = Vec::new();
    let max_samples = sample_rate as usize * 45; // batasi analisis pada 45 detik pertama untuk Reels/Shorts

    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track_id {
            continue;
        }
        if let Ok(audio_buf) = decoder.decode(&packet) {
            let spec = *audio_buf.spec();
            let duration = audio_buf.capacity() as u64;
            let mut sample_buf = SampleBuffer::<f32>::new(duration, spec);
            sample_buf.copy_interleaved_ref(audio_buf);

            let raw = sample_buf.samples();
            for frame in raw.chunks(channels) {
                let avg: f32 = frame.iter().copied().sum::<f32>() / (frame.len() as f32);
                mono_samples.push(avg);
            }
            if mono_samples.len() >= max_samples {
                break;
            }
        }
    }

    if mono_samples.is_empty() {
        return Err(anyhow!("File audio kosong atau tidak dapat di-decode"));
    }

    Ok(DecodedMonoAudio {
        samples: mono_samples,
        sample_rate,
    })
}
