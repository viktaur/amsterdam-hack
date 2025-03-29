use actix::prelude::*;
use serde::{Serialize, Deserialize};
use spectrum_analyzer::FrequencySpectrum;
use std::collections::HashSet;
use std::time::Duration;
use std::{collections::VecDeque};
use std::sync::{Arc, Mutex};

use crate::websockets::WsActor;

/// Size of the buffer for UDP packets.
pub const BUFFER_SIZE: usize = 4096;
/// Window size for our signal analysis.
pub const WINDOW_SIZE: usize = 1000;
/// How often to compute and send drone detection probability (in ms).
pub const DETECTION_INTERVAL_MS: u64 = 500;
/// How many samples are collected per second.
pub const SAMPLE_RATE: f32 = 44_100.0; // This will probably be much higher
// Expected dominant frequency emitted by the drone.
pub const DRONE_FREQ: f32 = 5_000.0;

pub struct DetectionActor {
    signal_window: SignalWindow,
    subscribers: HashSet<Addr<WsActor>>,
}

impl DetectionActor {
    pub fn new() -> Self {
        Self {
            signal_window: SignalWindow::new(WINDOW_SIZE),
            subscribers: HashSet::new(),
        }
    }
}

impl Actor for DetectionActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // Start a periodic task every 500ms
        ctx.run_interval(std::time::Duration::from_millis(DETECTION_INTERVAL_MS), |act, _ctx| {
            // Simulate adding 1000 random samples. TODO replace this with real samples
            let samples: Vec<f32> = (0..1000).map(|_| rand::random::<f32>() * 2.0 - 1.0).collect();
            act.signal_window.add_samples(&samples);

            // Simulate a frequency spectrum (since spectrum_analyzer is undefined)
            let dummy_spectrum = FrequencySpectrumStub {
                data: vec![(DRONE_FREQ, 1.0), (1000.0, 0.5)], // Placeholder
            };

            // Compute matching score.
            let score = DetectionScore::calculate(dummy_spectrum, DRONE_FREQ, 100.0); // Bandwidth = 100 Hz

            // Notify all subscribers
            for subscriber in &act.subscribers {
                subscriber.do_send(score.clone());
            }
        });
    }
}

// Messages for subscribing and unsubscribing (specific to DetectionActor).
#[derive(Message)]
#[rtype(result = "()")]
pub struct Subscribe(pub Addr<WsActor>);

#[derive(Message)]
#[rtype(result = "()")]
pub struct Unsubscribe(pub Addr<WsActor>);

impl Handler<Subscribe> for DetectionActor {
    type Result = ();

    fn handle(&mut self, msg: Subscribe, _: &mut Self::Context) {
        self.subscribers.insert(msg.0);
    }
}

impl Handler<Unsubscribe> for DetectionActor {
    type Result = ();

    fn handle(&mut self, msg: Unsubscribe, _: &mut Self::Context) {
        self.subscribers.remove(&msg.0);
    }
}


// Stub for FrequencySpectrum since it's not fully defined. TODO get rid of this
struct FrequencySpectrumStub {
    data: Vec<(f32, f32)>,
}

impl FrequencySpectrumStub {
    fn data(&self) -> &[(f32, f32)] {
        &self.data
    }
}


/// Holds our RF signal data window. Includes the samples currently recorded and max size
/// of the buffer.
pub struct SignalWindow {
    samples: VecDeque<f32>,
    max_size: usize,
}

impl SignalWindow {
    fn new(max_size: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max_size),
            max_size
        }
    }

    // Add a single sample
    fn add_sample(&mut self, sample: f32) {
        if self.samples.len() >= self.max_size {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    fn add_samples(&mut self, samples: &[f32]) {
        for sample in samples {
            self.add_sample(*sample);
        }
    }
}

/// Detection results to send to the UI client. Contains the score and a timestamp of when
/// it was calculated.
#[derive(Message)]
#[rtype(result = "()")]
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct DetectionScore {
    score: f32,
    timestamp: u64
}

impl DetectionScore {
    fn calculate(
        spectrum: FrequencySpectrum,
        target_freq: f32,
        bandwidth: f32,
    ) -> Self {
        let mut score = 0f32;
        let mut total_power = 0f32;

        for &(freq, power) in spectrum.data() {
            let distance = (freq.val() - target_freq).abs();
            let weight = (-distance.powi(2) / (2.0 * bandwidth.powi(2))).exp(); // Gaussian decay
            score += power.val() * weight;
            total_power += power.val();
        }

        Self {
            score: if total_power > 0.0 { score / total_power } else { 0.0 },
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_millis() as u64,
        }
    }
}
