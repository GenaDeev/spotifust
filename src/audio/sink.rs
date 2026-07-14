use librespot::playback::audio_backend::{Sink, SinkError, SinkResult};
use librespot::playback::convert::Converter;
use librespot::playback::decoder::AudioPacket;
use rodio::Sink as RodioSink;
use rodio::buffer::SamplesBuffer;
use tokio::sync::mpsc;

pub struct MpscSink {
    sender: mpsc::Sender<Vec<f32>>,
}

impl MpscSink {
    pub fn new(sender: mpsc::Sender<Vec<f32>>) -> Self {
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
            .blocking_send(vec_samples)
            .map_err(|e| SinkError::OnWrite(format!("Channel closed: {e}")))?;
        Ok(())
    }
}

pub fn spawn_rodio_thread(
    mut receiver: mpsc::Receiver<Vec<f32>>,
    rodio_sink: RodioSink,
    _stream: rodio::OutputStream,
) {
    std::thread::spawn(move || {
        // Continuously read PCM chunks from librespot and append them to rodio
        while let Some(samples) = receiver.blocking_recv() {
            // Librespot outputs stereo 44.1kHz by default
            let source = SamplesBuffer::new(2, 44100, samples);
            rodio_sink.append(source);
        }
    });
}
