extern crate hound;
extern crate sample;
extern crate velvet_noise;

fn convolve_kern(samples: &[f32], kern: &[(usize, f32)]) -> f32 {
    kern.iter()
        .map(|(i, x)| samples[*i] * x)
        .sum::<f32>()
}


/// Create an endless sound as decribed in http://dafx.de/paper-archive/2018/papers/DAFx2018_paper_11.pdf
pub fn main() {
    let mut reader = hound::WavReader::open("guitar_chord_mono_2.wav").unwrap();
    let samples = reader.samples::<i32>()
        .map(|s| sample::conv::i24::to_f32(sample::types::i24::I24::new_unchecked(s.unwrap())))
        .collect::<Vec<f32>>();
    
    // This is used for subsequent convolution coefficients
    let mut choice = velvet_noise::Choice::classic();

    // Create 10 seconds of audio
    let n_seconds = 10;
    let n_samples = reader.spec().sample_rate * n_seconds;
    let sample_rate = reader.spec().sample_rate as f32;
    let duration_s = reader.duration() as f32 / sample_rate;

     // paper suggests 32 simultaneous taps
    let density = 32. / duration_s;

    let mut initial_taps = velvet_noise::VelvetNoiseKernel(
        velvet_noise::OVNImpulseLocations::new(density as usize, sample_rate as usize),
        velvet_noise::Choice::classic()
    )
        .take_while(|(i, _)| i < &samples.len())
        .collect::<Vec<(usize, f32)>>();
    
    // output
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sample_rate as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create("out.wav", spec).unwrap();
    
    let max_index = samples.len() - 1;
    let gain = 0.3;
    
    for _ in 0..n_samples {
        writer.write_sample(convolve_kern(&samples, &initial_taps) * gain).unwrap();
        
        for tap in initial_taps.iter_mut() {
            *tap = (tap.0 + 1, tap.1);
        }

        // move taps along delay lines
        let length_before = initial_taps.len();
        initial_taps.retain(|&tap| tap.0 <= max_index);
        let length_after = initial_taps.len();
        for _ in 0..(length_before - length_after) {
            initial_taps.push((0, choice.next().unwrap()));
        }
    }

    writer.finalize().unwrap();
}