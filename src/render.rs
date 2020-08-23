use crate::{ViewBounds, WINDOW_SIZE};
use ggez::mint::Point2;
use num_complex::Complex32;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rayon::prelude::*;
use std::ops::{Add, Mul};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;

/// The maximum length^2 that will be used in the calculations
const COMPLEX_MAX_LEN_SQR: f32 = 10000.0;
const MAX_COMPLEX_ITERATIONS: i32 = 100;

type ComplexIterations = Option<i32>;
#[derive(Debug)]
pub struct Point {
    c: Complex32,
    pub i: ComplexIterations,
    pub p: Point2<f32>,
}

pub fn start_thread(bounds: ViewBounds) -> Receiver<Point> {
    let (sender, receiver) = mpsc::channel();
    // Worker thread to not block the UI
    thread::spawn(move || {
        // All pixel positions
        let mut pixels: Vec<(f32, f32)> = (0..WINDOW_SIZE)
            .flat_map(|x| (0..WINDOW_SIZE).map(move |y| (x as f32, y as f32)))
            .collect();
        pixels.shuffle(&mut thread_rng());
        let _result = pixels
            .par_iter()
            .map(|(pixel_x, pixel_y)| {
                let x = (pixel_x / WINDOW_SIZE as f32) * (bounds.1.re - bounds.0.re) + bounds.0.re;
                let y = (pixel_y / WINDOW_SIZE as f32) * (bounds.1.im - bounds.0.im) + bounds.0.im;
                let z = Complex32::new(x, y);
                let iterations = complex_iterations(z);
                Point {
                    c: z,
                    i: iterations,
                    p: [*pixel_x, *pixel_y].into(),
                }
            })
            .try_for_each_with(sender, |s, item| s.send(item));
    });
    receiver
}

// Checks if a complex number diverges
fn complex_iterations(c: Complex32) -> ComplexIterations {
    let mut z = Complex32::new(0.0, 0.0);
    for i in 0..MAX_COMPLEX_ITERATIONS {
        z = z.mul(z).add(c);
        if z.norm_sqr() > COMPLEX_MAX_LEN_SQR {
            return Some(i);
        }
        if z.norm_sqr() == 0.0 {
            return None;
        }
    }
    None
}
