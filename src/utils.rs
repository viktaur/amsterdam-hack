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
