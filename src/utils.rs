use std::{collections::HashMap, fs::File};

use spectrum_analyzer::{error::SpectrumAnalyzerError, samples_fft_to_spectrum, scaling::divide_by_N_sqrt, FrequencySpectrum, FrequencyLimit};

pub fn parse_samples(data: &[u8]) -> Vec<f32> {
    data.chunks_exact(4) // Each 32-bit float is 4 bytes
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

pub fn compute_spectrum(
    samples: &[f32],
    sampling_rate: u32,
) -> Result<FrequencySpectrum, SpectrumAnalyzerError> {
    samples_fft_to_spectrum(samples, sampling_rate, FrequencyLimit::All, Some(&divide_by_N_sqrt))
}

pub fn classify_uav(
    spectrum: FrequencySpectrum,
    reference_data: &HashMap<String, FrequencySpectrum>
) -> (String, f32) {
    let mut best_match = String::from("Unknown");
    let mut best_score = 0.0;

    for (uav_name, ref_spectrum) in reference_data {
        let similarity_score = cosine_similarity(&spectrum, ref_spectrum);
        if similarity_score > best_score {
            best_score = similarity_score;
            best_match = uav_name.clone();
        }
    }

    (best_match, best_score)
}

pub fn wav_to_signal(file: File) -> Result<Vec<f32>, ()> {
    let (_, samples) = wav_io::read_from_file(file).map_err(|_| ())?;
    Ok(samples)
}

pub fn cosine_similarity(spectrum: &FrequencySpectrum, ref_spectrum: &FrequencySpectrum) -> f32 {
    let mut vec1 = HashMap::new();
    let mut vec2 = HashMap::new();

    // Store first spectrum's frequency-power mapping
    for &(freq, power) in spectrum.data() {
        vec1.insert(freq.val().to_bits(), power.val());
    }

    // Store second spectrum's frequency-power mapping
    for &(freq, power) in ref_spectrum.data() {
        vec2.insert(freq.val().to_bits(), power.val());
    }

    // Compute dot product and vector magnitudes
    let mut dot_product = 0.0;
    let mut norm1 = 0.0;
    let mut norm2 = 0.0;

    for (&freq_bits, &power1) in &vec1 {
        let power2 = *vec2.get(&freq_bits).unwrap_or(&0.0);
        dot_product += power1 * power2;
        norm1 += power1.powi(2);
    }

    for &power2 in vec2.values() {
        norm2 += power2.powi(2);
    }

    // Compute cosine similarity
    if norm1 > 0.0 && norm2 > 0.0 {
        dot_product / (norm1.sqrt() * norm2.sqrt())
    } else {
        0.0
    }
}
