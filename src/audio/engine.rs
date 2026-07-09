use rodio::{OutputStreamBuilder, Sink, Source};
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;

#[allow(dead_code)]
pub enum AudioCommand {
    Play,
    Pause,
}

pub struct AudioEngine;

impl AudioEngine {
    pub fn spawn() -> tokio_mpsc::Sender<AudioCommand> {
        let (ui_tx, mut ui_rx) = tokio_mpsc::channel::<AudioCommand>(16);

        // Bounded channel to prevent memory bloat (max 8 chunks in flight)
        // Satisfies AGENTS.md constraint: "Bound the channel (mpsc::channel(N))"
        let (pcm_tx, pcm_rx) = tokio_mpsc::channel::<Vec<f32>>(8);

        // 1. Async Decoder Task (Mock Sine Wave for Phase 3 scaffolding)
        tokio::spawn(async move {
            let mut is_playing = false;
            let mut phase: f32 = 0.0;
            let sample_rate = 44100.0;
            let freq = 440.0; // A4 note

            loop {
                // If not playing, await a command. If playing, check non-blocking.
                if is_playing {
                    while let Ok(cmd) = ui_rx.try_recv() {
                        match cmd {
                            AudioCommand::Play => is_playing = true,
                            AudioCommand::Pause => is_playing = false,
                        }
                    }
                } else if let Some(cmd) = ui_rx.recv().await {
                    match cmd {
                        AudioCommand::Play => is_playing = true,
                        AudioCommand::Pause => is_playing = false,
                    }
                } else {
                    break;
                }

                if is_playing {
                    // Generate a chunk of sine wave
                    let mut chunk = Vec::with_capacity(1024 * 2);
                    for _ in 0..1024 {
                        let sample = (phase * 2.0 * std::f32::consts::PI).sin() * 0.1; // 10% volume
                        chunk.push(sample); // L
                        chunk.push(sample); // R
                        phase = (phase + freq / sample_rate).fract();
                    }

                    // Send backpressure: if rodio is slow, this await slows the generator
                    if pcm_tx.send(chunk).await.is_err() {
                        break; // Sink closed
                    }
                }
            }
        });

        // 2. OS Audio Thread (rodio Sink)
        std::thread::spawn(move || {
            let stream =
                OutputStreamBuilder::open_default_stream().expect("Failed to open audio output");
            let sink = Sink::connect_new(stream.mixer());

            let source = PcmSource {
                rx: pcm_rx,
                current_chunk: vec![],
                index: 0,
            };
            sink.append(source);

            sink.sleep_until_end();
        });

        ui_tx
    }
}

// Custom Rodio Source to consume PCM arrays from the bounded channel
struct PcmSource {
    rx: tokio_mpsc::Receiver<Vec<f32>>,
    current_chunk: Vec<f32>,
    index: usize,
}

impl Iterator for PcmSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.current_chunk.len() {
            // Block audio thread until tokio gives us PCM data
            let chunk = self.rx.blocking_recv()?;
            self.current_chunk = chunk;
            self.index = 0;
        }

        let sample = self.current_chunk[self.index];
        self.index += 1;
        Some(sample)
    }
}

impl Source for PcmSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        44100
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
