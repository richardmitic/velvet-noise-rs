# Velvet Noise

Implementation of several types of Velvet Noise as described in http://dafx.de/paper-archive/2019/DAFx2019_paper_53.pdf

An example application implementing the Signal Extrapolation technique described in http://dafx.de/paper-archive/2018/papers/DAFx2018_paper_11.pdf is also provided

## Usage

### Generate raw velvet noise audio
Example: generate 1s of velvet noise audio as a vector of `f32`s

```
let density = 2000.;
let sample_rate = 44100.;
let noise = original_velvet_noise(density, sample_rate);
let samples: Vec<f32> = noise.take(44100).collect();
```

### Generate a velvet convolution kernel
The unique property of velvet noise is that the majority of samples are `0.` and can hence be ignored during a convolution. To that end, this crate provides velvet convolution kernels - iterators yielding `(usize, f32)` where the first element is the index of the non-zero samples and the second element is either `1.` or `-1.`;