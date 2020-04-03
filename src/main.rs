extern crate hound;
extern crate sample;
extern crate velvet_noise;

use std::{io, fs};
use sample::{signal, Sample, Frame, Signal, I24};
use hound::WavReader;

fn convolve_kern(samples: &[f32], kern: &[(usize, f32)]) -> f32 {
    kern.iter().map(|(i, x)| samples[*i] * x).sum::<f32>()
}

/// Create an endless sound as decribed in http://dafx.de/paper-archive/2018/papers/DAFx2018_paper_11.pdf
fn process<O>(reader: WavReader<io::BufReader<fs::File>>) 
where
    O: Frame<Sample=f32>
{

    // read samples from file
    // TODO: make this generic over channels and sample type
    let spec = reader.spec().clone();
    let duration = reader.duration();
    let sample_iter = reader.into_samples().filter_map(Result::ok).map(|x| I24::new_unchecked(x).to_sample::<f32>());
    let sample_signal = signal::from_interleaved_samples_iter::<_, O>(sample_iter);
    let samples = sample_signal.until_exhausted().map(|frame| *frame.channel(0).unwrap()).collect::<Vec<f32>>();

    // Create 10 seconds of audio
    let n_seconds = 10;
    let n_samples = spec.sample_rate * n_seconds;
    let sample_rate = spec.sample_rate as f32;
    let duration_s = duration as f32 / sample_rate;

    // paper suggests 32 simultaneous taps
    let density = 32. / duration_s;

    // initialise an array of delay taps
    let mut taps = velvet_noise::VelvetNoiseKernel(
        velvet_noise::OVNImpulseLocations::new(density as usize, sample_rate as usize),
        velvet_noise::Choice::classic(),
    )
    .take_while(|(i, _)| i < &samples.len())
    .collect::<Vec<(usize, f32)>>();

    // used for subsequent convolution coefficients since the previous kernel is already taken
    let mut choice = velvet_noise::Choice::classic();

    // output
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sample_rate as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create("out.wav", spec).unwrap();

    let max_index = samples.len() - 1;
    let gain = 0.1;

    for _ in 0..n_samples {
        writer
            .write_sample(convolve_kern(&samples, &taps) * gain)
            .unwrap();

        for tap in taps.iter_mut() {
            *tap = (tap.0 + 1, tap.1);
        }

        // move taps along delay line
        let length_before = taps.len();
        taps.retain(|&tap| tap.0 <= max_index);
        let length_after = taps.len();
        for _ in 0..(length_before - length_after) {
            taps.push((0, choice.next().unwrap()));
        }
    }

    writer.finalize().unwrap();
}

pub fn main() {
    let reader = WavReader::open("guitar_chord_stereo.wav").unwrap();
    let channels = reader.spec().channels;
    match channels {
        1 => process::<[f32; 1]>(reader),
        2 => process::<[f32; 2]>(reader),
        _ => {}
    }
}