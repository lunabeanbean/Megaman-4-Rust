//! Sound output for the window: the flow produces mono samples (one video frame at a time), a ring buffer hands them to the sound card.
//! The window runs at about 60 frames per second and the console at 60.0988, so the buffer drifts; the callback repeats the last sample when it runs dry and the
//! producer side drops the oldest samples when too much has piled up (never more than a few frames of delay).
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub struct AudioOut {
    queue: Arc<Mutex<VecDeque<f32>>>,
    pub rate: u32,
    _stream: cpal::Stream,
    /// The most samples to keep queued (about 100 ms).
    cap: usize,
}

impl AudioOut {
    /// Opens the default output device; None (and a note on stderr) when there is none.
    pub fn start() -> Option<AudioOut> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = match device.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("no audio: {e}");
                return None;
            }
        };
        let rate = config.sample_rate();
        let queue: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::new()));
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => build::<f32>(&device, config.into(), queue.clone()),
            cpal::SampleFormat::I16 => build::<i16>(&device, config.into(), queue.clone()),
            cpal::SampleFormat::U16 => build::<u16>(&device, config.into(), queue.clone()),
            cpal::SampleFormat::I32 => build::<i32>(&device, config.into(), queue.clone()),
            f => {
                eprintln!("no audio: unsupported sample format {f}");
                return None;
            }
        };
        let stream = match stream {
            Some(s) => s,
            None => return None,
        };
        if let Err(e) = stream.play() {
            eprintln!("no audio: {e}");
            return None;
        }
        Some(AudioOut { queue, rate, _stream: stream, cap: rate as usize / 10 })
    }

    /// Adds samples (mono, at `rate`).
    pub fn push(&self, samples: &[f32]) {
        let mut q = self.queue.lock().unwrap();
        q.extend(samples.iter().copied());
        while q.len() > self.cap {
            q.pop_front();
        }
    }
}

fn build<T: SizedSample + FromSample<f32> + Send + 'static>(device: &cpal::Device, config: cpal::StreamConfig, queue: Arc<Mutex<VecDeque<f32>>>) -> Option<cpal::Stream> {
    let channels = config.channels as usize;
    let mut last = 0.0f32;
    let err_fn = |e: cpal::Error| eprintln!("audio stream error: {e}");
    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut q = queue.lock().unwrap();
            for frame in data.chunks_mut(channels) {
                if let Some(s) = q.pop_front() {
                    last = s;
                } else {
                    last *= 0.995; // dry: fade out the last value instead of a click
                }
                for c in frame.iter_mut() {
                    *c = T::from_sample(last);
                }
            }
        },
        err_fn,
        None,
    );
    match stream {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("no audio: {e}");
            None
        }
    }
}
