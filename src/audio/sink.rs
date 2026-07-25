use librespot::playback::audio_backend::{Sink, SinkError, SinkResult};
use librespot::playback::convert::Converter;
use librespot::playback::decoder::AudioPacket;
use rodio::Sink as RodioSink;
use rodio::buffer::SamplesBuffer;
use std::sync::mpsc::{Receiver, SyncSender};

pub struct MpscSink {
    sender: SyncSender<Vec<f32>>,
}

impl MpscSink {
    pub fn new(sender: SyncSender<Vec<f32>>) -> Self {
        Self { sender }
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
            .send(vec_samples)
            .map_err(|e| SinkError::OnWrite(format!("Channel closed: {e}")))?;
        Ok(())
    }
}

pub fn spawn_rodio_thread(
    receiver: Receiver<Vec<f32>>,
    rodio_sink: std::sync::Arc<RodioSink>,
    stream: rodio::OutputStream,
) {
    std::thread::spawn(move || {
        let _stream_guard = stream;
        while let Ok(samples) = receiver.recv() {
            if !samples.is_empty() {
                let source = SamplesBuffer::new(2, 44100, samples);
                rodio_sink.append(source);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::sync_channel;
    use std::time::Duration;

    #[test]
    fn test_mpsc_sink_bounded_backpressure() {
        let capacity = 8;
        let (tx, rx) = sync_channel::<Vec<f32>>(capacity);
        let sink = MpscSink::new(tx);

        let sent_count = Arc::new(AtomicUsize::new(0));
        let sent_count_clone = Arc::clone(&sent_count);

        let handle = std::thread::spawn(move || {
            let chunk = vec![0.0_f32; 2048];
            for _ in 0..100 {
                if sink.sender.send(chunk.clone()).is_ok() {
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
}
