extern crate num;

use num::complex::Complex64;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

pub trait Fractal: Send + Sync + Display {
    /// Calculate the fractal value at a specific point on the complex plane.
    /// return a normalized value
    fn calc(&self, z: Complex64) -> f32;

    fn change_maxiter(&self, &Fn(u32) -> u32) -> Arc<Box<Fractal>>;
}

fn julia(z: Complex64, c: Complex64, max_i: u32) -> u32 {
    let mut z = z;
    for i in 0..max_i {
        if z.norm_sqr() > 4.0 {
            return i;
        } else {
            z = z * z + c;
        }
    }
    0
}

pub struct Mandelbrot {
    maxiter: u32,
}

impl Mandelbrot {
    pub fn new(maxiter: u32) -> Arc<Box<Fractal>> {
        Arc::new(Box::new(Mandelbrot { maxiter: maxiter }))
    }
}
impl Fractal for Mandelbrot {
    fn calc(&self, z: Complex64) -> f32 {
        let zero = Complex64 { re: 0.0, im: 0.0 };
        julia(zero, z, self.maxiter) as f32 / self.maxiter as f32
    }
    fn change_maxiter(&self, f: &Fn(u32) -> u32) -> Arc<Box<Fractal>> {
        Arc::new(Box::new(Mandelbrot { maxiter: f(self.maxiter) }))
    }
}
impl Display for Mandelbrot {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Mandelbrot ({})", self.maxiter)
    }
}

pub struct Julia {
    c: Complex64,
    maxiter: u32,
}
impl Julia {
    pub fn new(c: Complex64, maxiter: u32) -> Arc<Box<Fractal>> {
        Arc::new(Box::new(Julia {
                              c: c,
                              maxiter: maxiter,
                          }))
    }
}
impl Fractal for Julia {
    fn calc(&self, z: Complex64) -> f32 {
        julia(z, self.c, self.maxiter) as f32 / self.maxiter as f32
    }
    fn change_maxiter(&self, f: &Fn(u32) -> u32) -> Arc<Box<Fractal>> {
        Arc::new(Box::new(Julia {
                              c: self.c,
                              maxiter: f(self.maxiter),
                          }))
    }
}

impl Display for Julia {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Julia {} ({})", self.c, self.maxiter)
    }
}
