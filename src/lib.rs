extern crate rand;

use rand::{Rng, SeedableRng};
use rand::rngs::{SmallRng, ThreadRng};
use rand::distributions::{Bernoulli, Distribution};

/// Original Velvet Noise impulse location iterator
pub struct OVNImpulseLocations {
    m: std::ops::RangeFrom<usize>,
    td: usize,
    r1m: SmallRng
}

impl OVNImpulseLocations {
    /// density is non-zero pulses per second
    /// sample_rate is total samples per second
    pub fn new(density: usize, sample_rate: usize) -> OVNImpulseLocations {
        OVNImpulseLocations {
            m: (0..),
            td: sample_rate / density,
            r1m: SmallRng::from_entropy()
        }
    }
}

impl Iterator for OVNImpulseLocations {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let val = (self.m.next().unwrap() * self.td) + self.r1m.gen_range(0, self.td);
        Some(val)
    }
}


/// Additive Random Noise impulse location iterator
pub struct ARNImpulseLocations {
    m_prev: f32,
    td_minus_1: f32,
    delta: f32,
    r1m: ThreadRng
}

impl ARNImpulseLocations {
    /// density is non-zero pulses per second
    /// sample_rate is total samples per second
    pub fn new(density: f32, sample_rate: f32, delta: f32) -> ARNImpulseLocations {
        ARNImpulseLocations {
            m_prev: 0.,
            td_minus_1: (sample_rate / density) - 1.,
            delta: delta,
            r1m: rand::thread_rng()
        }
    }
}

impl Iterator for ARNImpulseLocations {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let val = self.m_prev + 1. + (self.td_minus_1 * (1. - self.delta)) + (2. * self.delta * self.td_minus_1 * self.r1m.gen::<f32>());
        self.m_prev = val;
        Some(val as usize)
    }
}


/// Impulse indexes that wrap around a given buffer size
pub struct ChunkedOVNImpulseLocations {
    impulses: OVNImpulseLocations,
    chunk_length: usize,
    base: usize,
    store: Option<usize>
}

impl ChunkedOVNImpulseLocations {
    pub fn new(density: usize, sample_rate: usize, chunk_length: usize) -> ChunkedOVNImpulseLocations {
        ChunkedOVNImpulseLocations {
            impulses: OVNImpulseLocations::new(density, sample_rate),
            chunk_length: chunk_length,
            base: 0,
            store: None
        }
    }
}

impl Iterator for ChunkedOVNImpulseLocations {
    type Item = Vec<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut chunk: Vec<usize> = vec![];
        
        if let Some(x) = self.store {
            chunk.push(x);
            self.store = None;
        }
        
        loop {
            let x = self.impulses.next().unwrap();
            if x - self.base < self.chunk_length {
                chunk.push(x);
            } else {
                self.store = Some(x);
                self.base += self.chunk_length;
                break;
            }
        }

        Some(chunk)
    }
}



/// Random sequence of negative/positive samples
struct Choice(Bernoulli, SmallRng);

impl Choice {
    /// Crushed (skewed) sample choice
    fn crushed(skew: f64) -> Choice {
        Choice(Bernoulli::new(skew).unwrap(), SmallRng::from_entropy())
    }

    /// Classic sample choice
    fn classic() -> Choice {
        Choice::crushed(0.5)
    }
}

impl Iterator for Choice {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.sample(&mut self.1) {
            true => Some(1.),
            false => Some(-1.)
        }
    }
}


/// Velvet Noise Kernal
/// Iterator that will generate (index, coefficient) pairs.
/// All indices not given in a pair are assumed to contain a 0 coefficient
struct VelvetNoiseKernal<T: Iterator<Item=usize>, U: Iterator<Item=f32>> (T, U);

impl <T, U> Iterator for VelvetNoiseKernal<T, U> where 
    T: Iterator<Item=usize>, 
    U: Iterator<Item=f32> 
{
    type Item = (usize, f32);

    fn next(&mut self) -> Option<Self::Item> {
        match (self.0.next(), self.1.next()) {
            (Some(i), Some(x)) => Some((i, x)),
            _ => None
        }
    }
}


/// Audio signal generated by the given impulse location iterator
struct VelvetNoise {
    impulses: OVNImpulseLocations,
    r2m: Choice,
    n: usize,
    kovn: usize
}

impl VelvetNoise {
    fn new(density: usize, sample_rate: usize) -> VelvetNoise {
        let mut imps = OVNImpulseLocations::new(density, sample_rate);
        let kovn = imps.next().unwrap();
        VelvetNoise {
            impulses: imps,
            r2m: Choice::classic(),
            n: 0,
            kovn: kovn
        }
    }
}

impl Iterator for VelvetNoise {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let value = match self.n == self.kovn {
            true => {
                self.kovn = self.impulses.next().unwrap();
                self.r2m.next()
            },
            false => Some(0.)
        };
        self.n += 1;
        value
    }
}


/// Crushed Original Velvet Noise
struct CrushedOriginalVelvetNoise {
    impulses: OVNImpulseLocations,
    r2m: Choice,
    n: usize,
    kovn: usize
}

impl CrushedOriginalVelvetNoise {
    fn new(density: usize, sample_rate: usize, p: f64) -> CrushedOriginalVelvetNoise {
        let mut imps = OVNImpulseLocations::new(density, sample_rate);
        let kovn = imps.next().unwrap();
        CrushedOriginalVelvetNoise {
            impulses: imps,
            r2m: Choice::crushed(p),
            n: 0,
            kovn: kovn
        }
    }
}

impl Iterator for CrushedOriginalVelvetNoise {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let value = match self.n == self.kovn {
            true => {
                self.kovn = self.impulses.next().unwrap();
                self.r2m.next()
            },
            false => Some(0.)
        };
        self.n += 1;
        value
    }
}


/// Crushed Additive Velvet Noise
struct CrushedAdditiveVelvetNoise {
    impulses: ARNImpulseLocations,
    r2m: Choice,
    n: usize,
    kovn: usize
}

impl CrushedAdditiveVelvetNoise {
    fn new(density: f32, sample_rate: f32, delta: f32, p: f64) -> CrushedAdditiveVelvetNoise {
        let mut imps = ARNImpulseLocations::new(density, sample_rate, delta);
        let kovn = imps.next().unwrap();
        CrushedAdditiveVelvetNoise {
            impulses: imps,
            r2m: Choice::crushed(p),
            n: 0,
            kovn: kovn
        }
    }
}

impl Iterator for CrushedAdditiveVelvetNoise {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let value = match self.n == self.kovn {
            true => {
                self.kovn = self.impulses.next().unwrap();
                self.r2m.next()
            },
            false => Some(0.)
        };
        self.n += 1;
        value
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use more_asserts::*;

    macro_rules! assert_close_enough {
        ($value:expr, $expected:expr, $range:expr) => ({
            let (value, expected, range) = (&($value), &($expected), &($range));
            assert_ge!(*value, *expected - *range);
            assert_le!(*value, *expected + *range);
        });
    }

    fn spread(data: &[f32]) -> f32 {
        let dev = (0..data.len() - 1)
            .map(|i| {
                (*data)[i + 1] as f32 - (*data)[i] as f32
            })
            .collect::<Vec<f32>>();
        
        let max = dev.iter().cloned().fold(f32::NAN, f32::max);
        let min = dev.iter().cloned().fold(f32::NAN, f32::min);
        max - min
    }

    #[test]
    fn window_size() {
        let vil = OVNImpulseLocations::new(441, 44100);
        assert_eq!(vil.td, 100);
    }

    #[test]
    fn iter_locations() {
        
        // Run iterator for a long time and check that the average impulse density is correct
        // density and sample rate from http://dafx.de/paper-archive/2019/DAFx2019_paper_53.pdf
        
        let density = 2000;
        let sample_rate = 96000;
        let seconds = 100;
        let until = sample_rate * seconds;
        
        let vil = OVNImpulseLocations::new(density, sample_rate);
        let num_impulses = vil.take_while(|loc| (*loc) < until).count();

        assert_eq!(num_impulses / seconds, density);
    }

    #[test]
    fn iter_arn_locations() {
        
        // Run iterator for a long time and check that the average impulse density is correct
        // density and sample rate from http://dafx.de/paper-archive/2019/DAFx2019_paper_53.pdf
        
        let density = 2000;
        let sample_rate = 96000;
        let seconds = 100;
        let until = sample_rate * seconds;

        let max_spread = (sample_rate as f32 / density as f32) * 2.;
        
        let locs1 = ARNImpulseLocations::new(density as f32, sample_rate as f32, 0.);
        let impulses1 = locs1.take_while(|loc| (*loc) < until).map(|x| x as f32).collect::<Vec<f32>>();
        assert_close_enough!(spread(impulses1.as_slice()), 0., 0.01);

        let locs2 = ARNImpulseLocations::new(density as f32, sample_rate as f32, 1.);
        let impulses2 = locs2.take_while(|loc| (*loc) < until).map(|x| x as f32).collect::<Vec<f32>>();
        assert_close_enough!(spread(impulses2.as_slice()), max_spread, 2.);
        
        let locs3 = ARNImpulseLocations::new(density as f32, sample_rate as f32, 0.5);
        let impulses3 = locs3.take_while(|loc| (*loc) < until).map(|x| x as f32).collect::<Vec<f32>>();
        assert_close_enough!(spread(impulses3.as_slice()), max_spread * 0.5, 2.);
    }

    #[test]
    fn iter_chunked_locations() {
        
        let density = 2000;
        let sample_rate = 96000;
        let seconds = 100;
        let until = sample_rate * seconds;
        let chunk_size = 960;
        
        let cvil = ChunkedOVNImpulseLocations::new(density, sample_rate, chunk_size);
        let num_impulses = cvil.take(until / chunk_size).flatten().count();
        
        assert_eq!(num_impulses / seconds, density);
    }

    #[test]
    fn single_chunk() {
        
        let density = 2000;
        let sample_rate = 96000;
        
        let chunk = ChunkedOVNImpulseLocations::new(density, sample_rate, 960).next().unwrap();
        println!("{:?}", chunk);
    }
    
    #[test]
    fn classic_choice_is_even() {
        let c = Choice::classic();
        let total: f32 = c.take(1_000_000).sum();
        assert_close_enough!(total / 1_000_000., 0., 0.01);
    }

    #[test]
    fn crushed_choice_can_skew_positive() {
        let c = Choice::crushed(0.75);
        let total: f32 = c.take(1_000_000).sum();
        assert_close_enough!(total / 1_000_000., 0.5, 0.01);
    }

    #[test]
    fn crushed_choice_can_skew_negative() {
        let c = Choice::crushed(0.25);
        let total: f32 = c.take(1_000_000).sum();
        assert_close_enough!(total / 1_000_000., -0.5, 0.01);
    }

    #[test]
    fn kernel_init() {
        let kern = VelvetNoiseKernal(
            OVNImpulseLocations::new(10, 20),
            Choice::classic()
        );

        for (i, x) in kern.skip(1).take(10) {
            assert_gt!(i, 0);
            assert_ne!(x, 0.);
        }
        
    }

    #[test]
    fn iter_noise_samples() {
        
        // Check that a snippet of velvet noise contains at least one each of -1. and 1., and that
        // the overall density is correct. We cannot assert the ratio of -1. to 1. since it's
        // determined by the rand crate.
        // density and sample rate from http://dafx.de/paper-archive/2019/DAFx2019_paper_53.pdf
        
        let density = 2000;
        let sample_rate = 96000;
        
        let noise = VelvetNoise::new(density, sample_rate);
        let samples: Vec<f32> = noise.take(sample_rate).collect();

        assert_eq!(samples.iter().map(|s| *s as i32).max(), Some(1));
        assert_eq!(samples.iter().map(|s| *s as i32).min(), Some(-1));
        assert_eq!(samples.iter().map(|s| (*s).abs()).sum::<f32>(), density as f32);

        // let spec = hound::WavSpec {
        //     channels: 1,
        //     sample_rate: sample_rate as u32,
        //     bits_per_sample: 32,
        //     sample_format: hound::SampleFormat::Float,
        // };
        // let mut writer = hound::WavWriter::create("iter_noise_samples.wav", spec).unwrap();
        // for s in samples.into_iter() {
        //     writer.write_sample(s);
        // }
    }

    #[test]
    fn iter_crushed_noise_samples() {
        
        // Check that a snippet of velvet noise contains at least one each of -1. and 1., and that
        // the overall density is correct. We cannot assert the ratio of -1. to 1. since it's
        // determined by the rand crate.
        // density and sample rate from http://dafx.de/paper-archive/2019/DAFx2019_paper_53.pdf
        
        let density = 8000;
        let sample_rate = 96000;
        let crush_factor = 0.75;
        
        let noise = CrushedOriginalVelvetNoise::new(density, sample_rate, crush_factor);
        let samples: Vec<f32> = noise.take(sample_rate).collect();

        assert_eq!(samples.iter().cloned().fold(f32::NAN, f32::max), 1.);
        assert_eq!(samples.iter().cloned().fold(f32::NAN, f32::min), -1.);
        assert_gt!(samples.iter().cloned().sum::<f32>(), 0.);

        // let spec = hound::WavSpec {
        //     channels: 1,
        //     sample_rate: sample_rate as u32,
        //     bits_per_sample: 32,
        //     sample_format: hound::SampleFormat::Float,
        // };
        // let mut writer = hound::WavWriter::create("iter_crushed_noise_samples.wav", spec).unwrap();
        // for s in samples.into_iter() {
        //     writer.write_sample(s);
        // }
    }

    #[test]
    fn iter_crushed_arn_noise_samples() {
        
        // Check that a snippet of velvet noise contains at least one each of -1. and 1., and that
        // the overall density is correct. We cannot assert the ratio of -1. to 1. since it's
        // determined by the rand crate.
        // density and sample rate from http://dafx.de/paper-archive/2019/DAFx2019_paper_53.pdf
        
        let density = 8000.;
        let sample_rate = 96000.;
        let delta = 0.5;
        let crush_factor = 0.95;
        
        let noise = CrushedAdditiveVelvetNoise::new(density, sample_rate, delta, crush_factor);
        let samples: Vec<f32> = noise.take(sample_rate as usize).collect();

        assert_eq!(samples.iter().cloned().fold(f32::NAN, f32::max), 1.);
        assert_eq!(samples.iter().cloned().fold(f32::NAN, f32::min), -1.);
        assert_gt!(samples.iter().cloned().sum::<f32>(), 0.);

        // let spec = hound::WavSpec {
        //     channels: 1,
        //     sample_rate: sample_rate as u32,
        //     bits_per_sample: 32,
        //     sample_format: hound::SampleFormat::Float,
        // };
        // let mut writer = hound::WavWriter::create("iter_crushed_arn_noise_samples.wav", spec).unwrap();
        // for s in samples.into_iter() {
        //     writer.write_sample(s);
        // }
    }
}
