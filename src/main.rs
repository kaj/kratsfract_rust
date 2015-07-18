extern crate num;
use num::complex::Complex;
use num::complex::Complex64;
use std::io::Write;
use std::fs::File;

fn julia(z : Complex64, c : Complex64, max_i : u8) -> u8 {
    let mut zz = z.clone();
    for i in 0..max_i {
    	if zz.norm_sqr() > 4.0 {
	    return i;
	} else {
	   zz = zz * zz + c;
	}
    }
    return 0;
}

fn main() {
    let width = 800 as i32;
    let height = 600 as i32;
    let scale = 2.5 as f64;
    let maxiter = 150 as u8;

    let s = scale / height as f64;
    let xform = |x : i32, y : i32| {
       return Complex64 {re: (x - width/2) as f64, im: (y - height/2) as f64}.scale(s);
    };

    let zero = Complex {re: 0.0, im: 0.0};
    //let c = Complex {re: -0.75, im: 0.12};

    let mut out = File::create("foo.pgm")
        .ok().expect("Failed to create file");
    out.write_fmt(format_args!("P5\n{} {}\n{}\n", width, height, maxiter))
        .ok();

    for y in 0..height {
        for x in 0..width {
            out.write(&[julia(zero, xform(x, y), maxiter)]).ok();
        }
    }
}
