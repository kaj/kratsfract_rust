extern crate cairo;
extern crate gdk;
extern crate gtk;
extern crate num;
extern crate time;

use cairo::Context;
use gdk::ColorSpace;
use gdk::Pixbuf;
use gdk::cairo_interaction::ContextExt;
use gdk::key;
use gtk::signal::Inhibit;
use gtk::signal::WidgetSignals;
use gtk::traits::ContainerTrait;
use gtk::traits::WidgetTrait;
use gtk::traits::WindowTrait;
use num::complex::Complex64;
use num::complex::Complex;
use time::precise_time_ns;
use std::sync::{Arc,Mutex};
use std::cmp::{min,max};
use num::Float;
use std::thread;
use std::sync::mpsc;

fn julia(z : Complex64, c : Complex64, max_i : u32) -> u32 {
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

struct Transform {
    o: Complex64,
    s: f64
}

impl Transform {
    fn new(center: Complex64, scale: f64, width: i32, height: i32)
           -> Transform {
        let s = scale / min(width, height) as f64;
        Transform {
            o: center - Complex64{re: s * width as f64,
                                  im: s * height as f64},
            s: s * 2.0
        }
    }
    fn xform(&self, x: i32, y: i32) -> Complex64 {
        Complex64{re: x as f64, im: y as f64}.scale(self.s) + self.o
    }
    fn xformf(&self, x: f64, y: f64) -> Complex64 {
        Complex64{re: x as f64, im: y as f64}.scale(self.s) + self.o
    }
}

fn hsl2rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    let m2 = if l < 0.5 { l * (s+1.0) } else { l + s - l * s };
    let m1 = l*2.0-m2;
    let r = hue_to_rgb(m1, m2, h+1.0/3.0);
    let g = hue_to_rgb(m1, m2, h    );
    let b = hue_to_rgb(m1, m2, h-1.0/3.0);
    (r, g, b)
}

fn hue_to_rgb(m1: f32, m2: f32, h: f32) -> f32 {
    let h = if h<0.0 { h+1.0 } else if h>1.0 { h-1.0 } else { h };
    if h*6.0<1.0 { m1+(m2-m1)*h*6.0 }
    else if h*2.0<1.0 { m2 }
    else if h*3.0<2.0 {  m1+(m2-m1)*(2.0/3.0-h)*6.0 }
    else { m1 }
}

/// A working or done rendering (image) of a given fractal.
struct FractalRendering {
    width: i32,
    height: i32,
    xpos: i32,
    ypos: i32,
    receiver: mpsc::Receiver<(u8, u8, u8)>,
    image: gdk::Pixbuf
}
impl FractalRendering {
    fn new(width: i32, height: i32, xform: Transform, maxiter: u32) -> FractalRendering {
        //println!("Creating {}x{} rendering", width, height);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let start = precise_time_ns();
            let zero = Complex64{re: 0.0, im: 0.0};
            for y in 0..height {
                for x in 0..width {
                    let i = julia(zero, xform.xform(x, y), maxiter);
                    // Very simple palette ...
                    let (r, g, b) = {
                        if i == 0 {
                            (0, 0, 0)
                        } else {
                            let c = i as f32 / maxiter as f32;
                            let (r, g, b) = hsl2rgb(c, 1.0, c+0.1);
                            ((255.0 * r) as u8, (255.0*g) as u8, (255.0*b) as u8)
                        }
                    };
                    // TODO Stop rendering if listener is gone.
                    if tx.send((r, g, b)).is_err() {
                        println!("Stopping render after {} ms, receiver is gone",
                                (precise_time_ns() - start) / 1000000);
                        return;
                    }
                }
            }
            println!("Should render ... done in {} ms.",
                     (precise_time_ns() - start) / 1000000);
        });
        FractalRendering {
            width: width,
            height: height,
            xpos: 0,
            ypos: 0,
            receiver: rx,
            image: unsafe { gdk::Pixbuf::new(ColorSpace::RGB, false, 8, width, height) }.unwrap()
        }
    }
    /// Receive rendered pixels into the image.
    /// Returns true if the image is completley rendered, false otherwise.
    fn do_receive(&mut self) -> bool {
        if (self.xpos < self.width) || (self.ypos < self.height) {
            let n_channels = self.image.get_n_channels();
            let rowstride = self.image.get_rowstride();
            let data = unsafe { self.image.get_pixels() };
            //println!("Receive from ({}, {}) of {}x{}",
            //         self.xpos, self.ypos, self.width, self.height);
            for y in self.ypos..self.height {
                for x in self.xpos..self.width {
                    match self.receiver.try_recv() {
                        Ok((r, g, b)) => {
                            let pos = (y * rowstride + x * n_channels) as usize;
                            data[pos] = r;
                            data[pos + 1] = g;
                            data[pos + 2] = b;
                        },
                        _ => {
                            //println!("Reached ({}, {}) of {}x{}",
                            //         x, y, self.width, self.height);
                            self.xpos = x;
                            self.ypos = y;
                            return false;
                        }
                    }
                }
                self.xpos = 0;
            }
            //println!("Full image received!");
            self.xpos = self.width;
            self.ypos = self.height;
        }
        true
    }
}
impl Drop for FractalRendering {

    fn drop(&mut self) {
        //println!("Done with {}x{} rendering", self.width, self.height);
    }
}

struct FractalWidget {
    widget: gtk::DrawingArea,
    maxiter: u32,
    scale: f64,
    center: Complex64,
    rendering: Option<Mutex<FractalRendering>>
}

impl FractalWidget {
    fn new() -> Arc<Mutex<FractalWidget>> {
        let area = gtk::DrawingArea::new().unwrap();
        let result = Arc::new(Mutex::new(FractalWidget {
            widget: area,
            maxiter: 150,
            scale: 1.2,
            center: Complex{re: -0.5, im: 0.0},
            rendering: None
        }));
        let r2 = result.clone();
        result.lock().unwrap().widget.connect_draw(move |_w, c| r2.lock().unwrap().redraw(c));
        result
    }
    fn get_title(&self) -> String {
        format!("{} @ {} ±{:e} (max {})", "Mandelbrot",
                self.center, self.scale, self.maxiter)
    }
    fn zoom(&mut self, z: Complex64, s: f64) {
        self.center = z;
        self.scale *= s;
        self.rendering = None;
        self.widget.queue_draw();
    }
    fn inc_maxiter(&mut self) {
        let ten = 10.0_f64;
        self.maxiter += ten.powi(max(0, ((self.maxiter / 3) as f64).log10() as i32)) as u32;
        println!("Maxiter is {}", self.maxiter);
        self.rendering = None;
        self.widget.queue_draw();
    }
    fn dec_maxiter(&mut self) {
        let ten = 10.0_f64;
        self.maxiter =
            max(self.maxiter - ten.powi(max(0, ((self.maxiter / 3) as f64).log10() as i32)) as u32,
                1);
        println!("Maxiter is {}", self.maxiter);
        self.rendering = None;
        self.widget.queue_draw();
    }
    fn get_xform(&self) -> Transform {
        Transform::new(self.center, self.scale,
                       self.widget.get_allocated_width(),
                       self.widget.get_allocated_height())
    }

    fn redraw(&mut self, c : Context) -> Inhibit {
        //let start = precise_time_ns();
        //println!("redraw ...");
        let (rendered_width, rendered_height) =
            match self.rendering {
                Some(ref r) => { let rl = r.lock().unwrap(); (rl.width, rl.height) },
                _ =>       (0, 0)
            };
        let width = self.widget.get_allocated_width();
        let height = self.widget.get_allocated_height();
        if rendered_width != width || rendered_height != height {
            self.rendering = Some(Mutex::new(
                FractalRendering::new(width, height, self.get_xform(), self.maxiter)));
        };
        match self.rendering {
            Some(ref r) => {
                let mut renderer = r.lock().unwrap();
                let done = renderer.do_receive();
                let ref image = renderer.image;
                c.set_source_pixbuf(&image, 0.0, 0.0);
                c.rectangle(0.0, 0.0, image.get_width() as f64, image.get_height() as f64);
                c.fill();
                if !done {
                    self.widget.queue_draw();
                }
            }
            _ => ()
        };
        //println!("redraw ... done in {} ms.",
        //         (precise_time_ns() - start) / 1000000);
        Inhibit(true)
    }
}

fn main() {
    gtk::init().ok();
    let window = gtk::Window::new(gtk::WindowType::TopLevel).unwrap();
    window.set_default_size(800, 600);
    window.set_window_position(gtk::WindowPosition::Center);

    let area = FractalWidget::new();
    {
        let a = area.lock().unwrap();
        window.add(&a.widget);
        window.set_title(&a.get_title());
    }
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(true)
    });
    window.connect_key_press_event(|_w, e| {
        println!("Key pressed: {:?}", e._type);
        Inhibit(true)
    });
    let a1 = area.clone();
    let w = window.clone();
    window.connect_key_release_event(move |_w, e| {
        println!("{:?}: {}", e._type, e.keyval);
        match e.keyval {
            key::Escape => gtk::main_quit(),
            key::plus => {
                let mut a = a1.lock().unwrap();
                a.inc_maxiter();
                w.set_title(&a.get_title());
            }
            key::minus => {
                let mut a = a1.lock().unwrap();
                a.dec_maxiter();
                w.set_title(&a.get_title());
            }
            key::m => {
                let mut a = a1.lock().unwrap();
                let s = a.scale;
                a.maxiter = 100;
                a.zoom(Complex{re: -0.5, im: 0.0},
                       1.2 / s);
                w.set_title(&a.get_title());
            },
            _ => ()
        }
        Inhibit(true)
    });
    let a2 = area.clone();
    let w2 = window.clone();
    window.connect_button_release_event(move |_w, e| {
        let mut a = a2.lock().unwrap();
        let z = a.get_xform().xformf(e.x, e.y);
        println!("{:?} at {}", e._type, z);
        match e.button {
            1 => a.zoom(z, 0.5),
            2 => println!("should toggle julia"),
            3 => a.zoom(z, 2.0),
            _ => ()
        }
        w2.set_title(&a.get_title());
        Inhibit(true)
    });

    window.show_all();
    gtk::main();
}
