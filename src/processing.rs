use actix::prelude::*;
use log::{error, warn, info};
use serde::{Serialize, Deserialize};
use spectrum_analyzer::FrequencySpectrum;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::time::{Duration, SystemTime};
use std::{collections::VecDeque};
use std::sync::{Arc, Mutex};

use crate::websockets::{WsActor, InfoMsg};
use crate::utils::{classify_uav, compute_spectrum, wav_to_signal};

/// Window size for our signal analysis
pub const WINDOW_SIZE: usize = 1000;
/// How often to compute and send drone detection probability (in ms).
pub const DETECTION_INTERVAL_MS: u64 = 500;
/// How many samples are collected per second.
pub const SAMPLE_RATE: u32 = 62_500; // This will probably be much higher
// Expected dominant frequency emitted by the drone. (These are the ones from the drill)
// pub const DRONE_FREQS: &[f32] = &[
//     10_000.0, 20_000.0, 30_000.0, 40_000.0, 51_000.0, 61_000.0, 71_0000.0, 81_000.0,
//     92_000.0, 102_000.0, 112_000.0, 122_000.0, 132_000.0
// ];

pub const UAV_DATA_PATH: &str = "drone_types.json";

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
                    let detection_info = DetectionInfo::calculate(spectrum, UAV_DATA_PATH, BANDWIDTH);

                    // Notify all subscribers
                    for subscriber in &act.subscribers {
                        subscriber.do_send(InfoMsg(detection_info.clone()));
                    }

                    info!("Detection info sent to all subscribers: {:?}", detection_info);

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

#[derive(Serialize, Deserialize)]
struct UAVInfo {
    name: String,
    audio_path: String,
}

impl UAVInfo {
    pub fn load_uav_reference_data(file_path: &str) -> Vec<Self> {
        let file_content = std::fs::read_to_string(file_path).expect("Failed to read UAV data file");
        serde_json::from_str(&file_content).expect("Failed to parse UAV JSON data")
    }
}

/// Detection results to send to the UI client. Contains the score, timestamp of when
/// it was calculated, and the closest drone match.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DetectionInfo {
    score: f32,
    timestamp: u64,
    uav_type: String,
}

impl DetectionInfo {
    pub fn new() -> Self {
        Self {
            score: 0.0,
            timestamp: 0,
            uav_type: "Unknown".into(),
        }
    }

    fn calculate(
        spectrum: FrequencySpectrum,
        uav_data_path: &str,
        bandwidth: f32
    ) -> Self {
        let uav_data = UAVInfo::load_uav_reference_data(uav_data_path);

        // Process stored UAV RF data into frequency spectra
        let mut uav_type_to_spectrum: HashMap<String, FrequencySpectrum> = HashMap::new();
        for uav in &uav_data {
            let signal = wav_to_signal(File::open(&uav.audio_path).expect("Could not open file"))
                .expect("Could not convert WAV to signal");
            let spectrum = compute_spectrum(&signal, SAMPLE_RATE).expect("Could not get spectrum");
            uav_type_to_spectrum.insert(uav.name.clone(), spectrum);
        }

        // Classify detected signal
        let (uav_type, score) = classify_uav(spectrum, &uav_type_to_spectrum);

        DetectionInfo {
            score,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_millis() as u64,
            uav_type,
        }
    }

//     fn calculate(
//         spectrum: FrequencySpectrum,
//         target_freqs: &[f32],
//         bandwidth: f32,
//     ) -> Self {
//         let mut score = 0f32;
//         let mut total_power = 0f32;

//         for &(freq, power) in spectrum.data() {
//             let mut max_weight = 0f32;

//             for &target_freq in target_freqs {
//                 let distance = (freq.val() - target_freq).abs();
//                 let weight = (-distance.powi(2) / (2.0 * bandwidth.powi(2))).exp(); // Gaussian decay
//                 // We only care about the freq with the max weight to ensure the most relevant frequency dominates
//                 max_weight = max_weight.max(weight);
//             }

//             score += power.val() * max_weight;
//             total_power += power.val();
//         }

//         Self {
//             score: if total_power > 0.0 { score / total_power } else { 0.0 },
//             timestamp: std::time::SystemTime::now()
//                 .duration_since(std::time::UNIX_EPOCH)
//                 .unwrap_or(Duration::from_secs(0))
//                 .as_millis() as u64,
//             uav_type
//         }
//     }
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
