use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

pub struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    stream: Option<cpal::Stream>,
    sample_rate: u32,
    channels: u16,
    pub is_recording: bool,
    pub has_sound: bool,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            stream: None,
            sample_rate: 16000,
            channels: 1,
            is_recording: false,
            has_sound: false,
        }
    }

    pub fn start(&mut self) -> Result<(), String> {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or("No input device found")?;

        // Use device's default config instead of hardcoding
        let default_config = device.default_input_config()
            .map_err(|e| format!("No default input config: {}", e))?;

        let config = cpal::StreamConfig {
            channels: default_config.channels(),
            sample_rate: default_config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        self.sample_rate = config.sample_rate.0;
        self.channels = config.channels;
        self.samples.lock().unwrap().clear();

        let samples = self.samples.clone();
        let channels = config.channels as usize;

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Downmix to mono if stereo
                if channels > 1 {
                    let mono: Vec<f32> = data.chunks(channels)
                        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
                        .collect();
                    samples.lock().unwrap().extend_from_slice(&mono);
                } else {
                    samples.lock().unwrap().extend_from_slice(data);
                }
            },
            |err| {
                eprintln!("[VOICE] Stream error: {}", err);
            },
            None,
        ).map_err(|e| format!("Failed to build stream: {}", e))?;

        stream.play().map_err(|e| format!("Failed to start stream: {}", e))?;
        self.stream = Some(stream);
        self.is_recording = true;
        Ok(())
    }

    pub fn stop(&mut self) -> Vec<u8> {
        self.stream = None;
        self.is_recording = false;

        let samples = self.samples.lock().unwrap();
        let duration = samples.len() as f32 / self.sample_rate as f32;
        let max_amp: f32 = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

        // Encode WAV — always mono
        let wav = encode_wav(&samples, self.sample_rate, 1);

        // Debug: save to disk and log stats
        let _ = std::fs::write("/tmp/claude-voice-debug.wav", &wav);
        crate::logging::log(&format!(
            "Voice stop: {} samples, rate={}, dur={:.1}s, max_amp={:.4}, wav={}b",
            samples.len(), self.sample_rate, duration, max_amp, wav.len()
        ));

        wav
    }

    /// Check if there's sound activity (RMS > threshold)
    pub fn check_activity(&self) -> bool {
        if let Ok(samples) = self.samples.lock() {
            let len = samples.len();
            if len < 1600 { return false; }
            let recent = &samples[len.saturating_sub(1600)..];
            let rms: f32 = (recent.iter().map(|s| s * s).sum::<f32>() / recent.len() as f32).sqrt();
            rms > 0.01
        } else {
            false
        }
    }
}

fn encode_wav(samples: &[f32], sample_rate: u32, channels: u16) -> Vec<u8> {
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    let block_align = channels * bits_per_sample / 8;
    let data_size = samples.len() as u32 * 2; // 16-bit = 2 bytes per sample

    let mut buf = Vec::new();
    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(36 + data_size).to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    // fmt chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());
    // data chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for &sample in samples {
        let s = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        buf.extend_from_slice(&s.to_le_bytes());
    }
    buf
}

/// Send WAV audio to Groq Whisper API for transcription
pub async fn transcribe_groq(wav_data: Vec<u8>, api_key: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    let part = reqwest::multipart::Part::bytes(wav_data)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;

    let form = reqwest::multipart::Form::new()
        .text("model", "whisper-large-v3-turbo")
        .text("language", "en")
        .part("file", part);

    let resp = client
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Groq API error {}: {}", status, body));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    json.get("text")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "No text in response".to_string())
}
