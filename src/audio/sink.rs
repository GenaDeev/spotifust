use crate::error::AppError;
use librespot::playback::audio_backend::{Sink, SinkError, SinkResult};
use librespot::playback::convert::Converter;
use librespot::playback::decoder::AudioPacket;
use rodio::Sink as RodioSink;
use rodio::buffer::SamplesBuffer;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender};

/// Interleaved stereo samples per second of librespot output (44.1 kHz × 2).
pub const SAMPLES_PER_SECOND: u64 = 44_100 * 2;

/// One decoded packet, tagged with the playback generation it was decoded for.
pub type PcmPacket = (u64, Vec<f32>);

/// Measures what the speakers have actually played, so the UI's position follows
/// audible audio rather than the decoder (which runs ahead) or a wall clock (which
/// kept running across pauses).
///
/// Every track load or seek starts a new *generation*. Packets are tagged with the
/// generation current when librespot decoded them, and the sample counter restarts
/// when the first sample of a new generation reaches the output.
#[derive(Debug, Default)]
pub struct PlaybackClock {
    /// Generation requested by the most recent load/seek.
    requested: AtomicU64,
    /// Packets from generations below this are stale (flushed by a seek/skip).
    discard_below: AtomicU64,
    /// Generation whose samples are currently coming out of the speakers.
    playing: AtomicU64,
    /// Samples of `playing` emitted so far.
    samples: AtomicU64,
}

impl PlaybackClock {
    /// Starts a new generation. With `flush`, packets of older generations still in
    /// flight are dropped instead of played.
    pub fn next_generation(&self, flush: bool) -> u64 {
        let generation = self.requested.fetch_add(1, Ordering::SeqCst) + 1;
        if flush {
            self.discard_below.store(generation, Ordering::SeqCst);
        }
        generation
    }

    #[must_use]
    pub fn requested(&self) -> u64 {
        self.requested.load(Ordering::SeqCst)
    }

    /// Milliseconds of `generation` played so far, or `None` if its audio
    /// hasn't reached the speakers yet.
    #[must_use]
    pub fn played_ms(&self, generation: u64) -> Option<u32> {
        if self.playing.load(Ordering::SeqCst) != generation {
            return None;
        }
        let samples = self.samples.load(Ordering::Relaxed);
        u32::try_from(samples * 1000 / SAMPLES_PER_SECOND).ok()
    }

    fn is_stale(&self, generation: u64) -> bool {
        generation < self.discard_below.load(Ordering::SeqCst)
    }
}

/// Wraps a packet so the clock advances only as rodio actually pulls samples.
struct CountingSource {
    inner: SamplesBuffer,
    generation: u64,
    started: bool,
    clock: Arc<PlaybackClock>,
}

impl Iterator for CountingSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.inner.next()?;
        if !self.started {
            self.started = true;
            if self.clock.playing.load(Ordering::SeqCst) != self.generation {
                self.clock.samples.store(0, Ordering::SeqCst);
                self.clock.playing.store(self.generation, Ordering::SeqCst);
            }
        }
        self.clock.samples.fetch_add(1, Ordering::Relaxed);
        Some(sample)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl rodio::Source for CountingSource {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.inner.channels()
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.inner.total_duration()
    }
}

/// Decoded packets allowed in rodio's queue (~20-45 ms each, so roughly 0.5-1 s).
const MAX_QUEUED_PACKETS: usize = 24;
const QUEUE_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(5);

pub struct MpscSink {
    sender: SyncSender<PcmPacket>,
    clock: Arc<PlaybackClock>,
}

impl MpscSink {
    pub fn new(sender: SyncSender<PcmPacket>, clock: Arc<PlaybackClock>) -> Self {
        Self { sender, clock }
    }
}

impl Sink for MpscSink {
    fn start(&mut self) -> SinkResult<()> {
        Ok(())
    }

    fn stop(&mut self) -> SinkResult<()> {
        Ok(())
    }

    fn write(&mut self, packet: AudioPacket, converter: &mut Converter) -> SinkResult<()> {
        let samples = packet
            .samples()
            .map_err(|e| SinkError::OnWrite(e.to_string()))?;
        let f32_samples: &[f32] = &converter.f64_to_f32(samples);

        let vec_samples = f32_samples.to_vec();
        self.sender
            .send((self.clock.requested(), vec_samples))
            .map_err(|e| SinkError::OnWrite(format!("Channel closed: {e}")))?;
        Ok(())
    }
}

pub fn spawn_rodio_thread(
    receiver: Receiver<PcmPacket>,
    clock: Arc<PlaybackClock>,
) -> Result<std::sync::Arc<RodioSink>, AppError> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let stream = match rodio::OutputStreamBuilder::from_default_device()
            .map_err(|e| AppError::Playback(format!("Failed to get default audio device: {e}")))
            .and_then(|builder| {
                builder
                    .open_stream()
                    .map_err(|e| AppError::Playback(format!("Failed to open audio stream: {e}")))
            }) {
            Ok(s) => s,
            Err(err) => {
                let _ = tx.send(Err(err));
                return;
            }
        };

        let rodio_sink = std::sync::Arc::new(RodioSink::connect_new(stream.mixer()));
        if tx.send(Ok(std::sync::Arc::clone(&rodio_sink))).is_err() {
            return;
        }

        let _stream_guard = stream;
        while let Ok((generation, samples)) = receiver.recv() {
            if samples.is_empty() || clock.is_stale(generation) {
                continue;
            }
            // Backpressure: librespot decodes far faster than real time, and
            // `rodio::Sink` queues without limit. Without this wait a whole track
            // (~75 MB of f32 PCM) piled up in memory, and EndOfTrack fired minutes
            // before the audio actually finished. Waiting here keeps the bounded
            // mpsc channel full, which in turn blocks the decoder.
            while rodio_sink.len() >= MAX_QUEUED_PACKETS {
                std::thread::sleep(QUEUE_POLL_INTERVAL);
            }
            rodio_sink.append(CountingSource {
                inner: SamplesBuffer::new(2, 44100, samples),
                generation,
                started: false,
                clock: Arc::clone(&clock),
            });
        }
    });

    rx.recv()
        .map_err(|_| AppError::Playback("Audio thread exited before initialization".into()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::sync::mpsc::sync_channel;
    use std::time::Duration;

    #[test]
    fn test_mpsc_sink_bounded_backpressure() {
        let capacity = 8;
        let (tx, rx) = sync_channel::<PcmPacket>(capacity);
        let sink = MpscSink::new(tx, Arc::new(PlaybackClock::default()));

        let sent_count = Arc::new(AtomicUsize::new(0));
        let sent_count_clone = Arc::clone(&sent_count);

        let handle = std::thread::spawn(move || {
            let chunk = vec![0.0_f32; 2048];
            for _ in 0..100 {
                if sink.sender.send((0, chunk.clone())).is_ok() {
                    sent_count_clone.fetch_add(1, Ordering::SeqCst);
                } else {
                    break;
                }
            }
        });

        std::thread::sleep(Duration::from_millis(50));

        let count_blocked = sent_count.load(Ordering::SeqCst);
        assert_eq!(
            count_blocked, capacity,
            "Bounded channel must block producer at capacity {capacity}, actual: {count_blocked}"
        );

        for _ in 0..3 {
            let _ = rx.recv();
        }

        std::thread::sleep(Duration::from_millis(50));
        let count_after_drain = sent_count.load(Ordering::SeqCst);
        assert_eq!(
            count_after_drain,
            capacity + 3,
            "Producer should unblock and send 3 more items, actual: {count_after_drain}"
        );

        drop(rx);
        let _ = handle.join();
    }

    #[test]
    fn test_clock_counts_only_played_samples_of_current_generation() {
        let clock = Arc::new(PlaybackClock::default());
        let generation = clock.next_generation(true);
        assert_eq!(clock.played_ms(generation), None, "nothing audible yet");

        let mut source = CountingSource {
            inner: SamplesBuffer::new(2, 44100, vec![0.0; 8820]),
            generation,
            started: false,
            clock: Arc::clone(&clock),
        };
        for _ in 0..4410 {
            source.next();
        }
        assert_eq!(clock.played_ms(generation), Some(50));

        let next = clock.next_generation(true);
        assert!(clock.is_stale(generation));
        assert_eq!(clock.played_ms(next), None);
    }
}
