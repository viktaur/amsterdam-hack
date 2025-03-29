use actix::prelude::*;
use log::{error, warn, info};
use serde::{Serialize, Deserialize};
use spectrum_analyzer::FrequencySpectrum;
use std::collections::HashSet;
use std::time::Duration;
use std::{collections::VecDeque};
use std::sync::{Arc, Mutex};

use crate::websockets::{WsActor, ScoreMsg};
use crate::utils::compute_spectrum;

/// Window size for our signal analysis
pub const WINDOW_SIZE: usize = 1000;
/// How often to compute and send drone detection probability (in ms).
pub const DETECTION_INTERVAL_MS: u64 = 500;
/// How many samples are collected per second.
pub const SAMPLE_RATE: u32 = 44_100; // This will probably be much higher
// Expected dominant frequency emitted by the drone.
pub const DRONE_FREQ: f32 = 5_000.0;

pub const BANDWIDTH: f32 = 0.0; // TODO define

pub struct ProcessingActor {
    signal_window: SignalWindow,
    subscribers: HashSet<Addr<WsActor>>,
}

impl ProcessingActor {
    pub fn new() -> Self {
        Self {
            signal_window: SignalWindow::new(WINDOW_SIZE),
            subscribers: HashSet::new(),
        }
    }

    pub fn get_samples(&self) -> Vec<f32> {
        self.signal_window.samples.clone().into()
    }

    pub fn clear_samples(&mut self) {
        self.signal_window.samples.clear();
    }
}

impl Actor for ProcessingActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_millis(DETECTION_INTERVAL_MS), |act, _| {
            match compute_spectrum(&act.get_samples(), SAMPLE_RATE) {
                Ok(spectrum) => {
                    let score = DetectionScore::calculate(spectrum, DRONE_FREQ, BANDWIDTH);

                    // Notify all subscribers
                    for subscriber in &act.subscribers {
                        subscriber.do_send(ScoreMsg(score));
                    }

                    info!("Score sent to all subscribers: {}", score.score);

                    act.clear_samples();
                },
                Err(err) => {
                    warn!("{:?}. No problem, retrying on next interval", err);
                }
            }
        });
    }
}


#[derive(Message)]
#[rtype(result = "()")]
pub struct Subscribe(pub Addr<WsActor>);

#[derive(Message)]
#[rtype(result = "()")]
pub struct Unsubscribe(pub Addr<WsActor>);


#[derive(Message)]
#[rtype(result = "()")]
pub struct AddSamples {
    pub samples: Vec<f32>
}


impl Handler<Subscribe> for ProcessingActor {
    type Result = ();

    fn handle(&mut self, msg: Subscribe, _: &mut Self::Context) {
        self.subscribers.insert(msg.0);
    }
}

impl Handler<Unsubscribe> for ProcessingActor {
    type Result = ();

    fn handle(&mut self, msg: Unsubscribe, _: &mut Self::Context) {
        self.subscribers.remove(&msg.0);
    }
}

impl Handler<AddSamples> for ProcessingActor {
    type Result = ();
    fn handle(&mut self, msg: AddSamples, _: &mut Self::Context) {
        self.signal_window.add_samples(&msg.samples);
    }
}


/// Detection results to send to the UI client. Contains the score and a timestamp of when
/// it was calculated.
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct DetectionScore {
    score: f32,
    timestamp: u64
}

impl DetectionScore {
    pub fn new() -> Self {
        Self {
            score: 0.0,
            timestamp: 0
        }
    }

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
