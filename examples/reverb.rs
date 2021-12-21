/// Implement the long-tail part of the reverb algorithm from
/// https://www.dafx.de/paper-archive/2013/papers/55.dafx2013_submission_54.pdf
use dasp_ring_buffer::Fixed;
use dasp_sample::Sample;
use dasp_signal::{from_interleaved_samples_iter, Signal};
use hound::WavReader;
use std::env;

use velvet_noise::{Choice, OVNImpulseLocations, VelvetNoiseKernel};

fn db_to_linear(db: f32) -> f32 {
    10f32.powf(db / 20.)
}

/// Schoeder allpass as in diagram at
/// https://ccrma.stanford.edu/~jos/pasp/Allpass_Two_Combs.html
/// b0 == aM == g
struct AllPass {
    buffer: Fixed<Vec<f32>>,
    delay_index: usize,
    g: f32,
}

impl AllPass {
    fn new(delay: usize, feedback: f32) -> Self {
        Self {
            buffer: Fixed::from(vec![0f32; delay]),
            delay_index: delay - 1,
            g: feedback,
        }
    }

    fn process(&mut self, sample: f32) -> f32 {
        let delay = *self.buffer.get(self.delay_index);
        let feedback = sample + (delay * -self.g);
        self.buffer.push(feedback);
        let feedforward = feedback * self.g;
        delay + feedforward
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: ./reverb <wav in> <wav out>");
        return;
    }
    let reader = WavReader::open(args[1].as_str()).unwrap();
    if reader.spec().channels != 1 {
        println!("Input file must be 1 channel");
        return;
    }

    let sample_rate = reader.spec().sample_rate;

    // Add 5s of tail time for reverb to fade
    let num_output_samples = reader.duration() as usize + (5 * sample_rate as usize);

    // Given on page 5, footnote 4
    let border_samples = vec![
        4411, 5672, 7214, 9044, 11171, 13602, 16343, 19400, 22779, 26484, 30521, 34895, 39609,
        44669, 50077, 55837, 61954, 68431, 75271, 82477, 90053,
    ];

    // Given on page 5
    let num_stages = 20usize;
    let max_density = 100;
    let min_density = 40;
    let density_step = (max_density - min_density) / num_stages;

    // Exact gains aren't specified. The paper just states that they are calculated from
    // the original imuplse response. This should be close enough based on figure 8.
    let max_gain_db = 0f32;
    let min_gain_db = -30f32;
    let gain_step_db = (max_gain_db - min_gain_db) / num_stages as f32;
    let first_stage_gain_boost = 3f32;

    // Make all 20 kernels
    let kernels: Vec<Vec<(usize, f32)>> = (0..num_stages)
        .into_iter()
        .map(|i| {
            let min_idx = border_samples[i];
            let max_idx = border_samples[i + 1];
            let density = max_density - (i * density_step);
            let gain = if i == 0 {
                db_to_linear(max_gain_db + first_stage_gain_boost)
            } else {
                db_to_linear(max_gain_db - (i as f32 * gain_step_db))
            };

            VelvetNoiseKernel::new(
                OVNImpulseLocations::new(density, sample_rate as usize),
                Choice::classic(),
            )
            .render(min_idx, max_idx, gain)
        })
        .collect();

    // Combine rendered kernels into a single one since they will all be convolved
    // with the same delay buffer.
    let combined_kernel: Vec<(usize, f32)> =
        kernels.into_iter().flat_map(|k| k.into_iter()).collect();

    // Convert original file to f32 samples
    let sample_iter = reader
        .into_samples::<i16>()
        .map(|s| s.unwrap().to_sample::<f32>());

    // Universal delay buffer
    let mut slice = [0f32; 100_000];
    let mut delay_buffer = Fixed::from(&mut slice[..]);
    let sample_signal = from_interleaved_samples_iter::<_, f32>(sample_iter);

    // Cascaded allpass filters, given on page 5
    let mut allpass_filters = vec![
        AllPass::new(1, 0.618),
        AllPass::new(64, 0.618),
        AllPass::new(140, 0.618),
        AllPass::new(209, 0.618),
        AllPass::new(442, 0.618),
        AllPass::new(555, 0.618),
        AllPass::new(630, 0.618),
    ];

    // output file
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(args[2].as_str(), spec).unwrap();

    // try not to clip any samples
    let output_gain = 0.2;

    // DSP
    for sample in sample_signal.take(num_output_samples) {
        delay_buffer.push(sample);
        let mut samp_out: f32 = combined_kernel
            .iter()
            .map(|(idx, gain)| delay_buffer.get(99_999 - *idx) * gain)
            .sum();

        for ap in allpass_filters.iter_mut() {
            samp_out = ap.process(samp_out);
        }

        samp_out *= output_gain;

        assert!(samp_out < 1.);
        writer.write_sample(samp_out).unwrap();
    }

    writer.finalize().unwrap();
}
