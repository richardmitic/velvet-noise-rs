extern crate velvet_noise;
use velvet_noise::VelvetImpulseLocations;

pub fn main() {
    let mut vil = VelvetImpulseLocations::new(4410, 44100);
    for i in 0..100 {
        println!("{} {}", i, vil.next().unwrap());
    }
}