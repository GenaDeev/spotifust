use librespot::playback::audio_backend::{Sink, SinkResult, SinkError};
use librespot::playback::decoder::AudioPacket;
use librespot::playback::convert::Converter;
use tokio::sync::mpsc;
use rodio::buffer::SamplesBuffer;
use rodio::Sink as RodioSink;

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
        let samples = packet.samples().map_err(|e| SinkError::OnWrite(e.to_string()))?;
        let f32_samples: &[f32] = &converter.f64_to_f32(samples);
        
        let vec_samples = f32_samples.to_vec();
        self.sender.blocking_send(vec_samples).map_err(|e| SinkError::OnWrite(format!("Channel closed: {e}")))?;
        Ok(())
    }
}

pub fn spawn_rodio_thread(mut receiver: mpsc::Receiver<Vec<f32>>) {
    std::thread::spawn(move || {
        let stream = rodio::OutputStreamBuilder::from_default_device()
            .expect("Failed to get default device")
            .open_stream()
            .expect("Failed to open stream");
        let rodio_sink = RodioSink::connect_new(stream.mixer());

        // Continuously read PCM chunks from librespot and append them to rodio
        while let Some(samples) = receiver.blocking_recv() {
            // Librespot outputs stereo 44.1kHz by default
            let source = SamplesBuffer::new(2, 44100, samples);
            rodio_sink.append(source);
        }
    });
}
