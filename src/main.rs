extern crate cairo;
extern crate gdk;
extern crate gdk_pixbuf;
extern crate gtk;
extern crate num;
extern crate time;

mod basicfractals;
mod palette;

use basicfractals::{Fractal, Julia, Mandelbrot};
use cairo::Context;
use gdk::{BUTTON1_MASK, BUTTON2_MASK, BUTTON3_MASK};
use gdk::enums::key;
use gdk::prelude::ContextExt;
use gdk_pixbuf::{Colorspace, Pixbuf};
use gtk::ContainerExt;
use gtk::Inhibit;
use gtk::WidgetExt;
use gtk::WindowExt;
use num::Zero;
use num::complex::{Complex, Complex64};
use palette::Palette;
use std::cmp::{max, min};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use time::precise_time_ns;

const GTK_COLORSPACE_RGB: Colorspace = 0; // TODO Import somewhere?

struct Transform {
    o: Complex64,
    s: f64,
}

impl Transform {
    fn new(center: Complex64,
           scale: f64,
           width: i32,
           height: i32)
           -> Transform {
        let s = scale / min(width, height) as f64;
        let change = Complex64 {
            re: s * width as f64,
            im: s * height as f64,
        };
        Transform {
            o: center - change,
            s: s * 2.0,
        }
    }
    fn xform(&self, x: i32, y: i32) -> Complex64 {
        self.xformf(x as f64, y as f64)
    }
    fn xformf(&self, x: f64, y: f64) -> Complex64 {
        Complex64 { re: x, im: y }.scale(self.s) + self.o
    }
}

/// A working or done rendering (image) of a given fractal.
struct FractalRendering {
    width: i32,
    height: i32,
    xpos: i32,
    ypos: i32,
    receiver: mpsc::Receiver<(u8, u8, u8)>,
    image: Pixbuf,
}

impl FractalRendering {
    fn new(width: i32,
           height: i32,
           xform: Transform,
           fractal: Arc<Box<Fractal>>,
           palette: Palette)
           -> FractalRendering {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let start = precise_time_ns();
            for y in 0..height {
                for x in 0..width {
                    let i = fractal.calc(xform.xform(x, y));
                    if tx.send(palette.color(i)).is_err() {
                        println!("Stopping render after {} ms, receiver gone",
                                 (precise_time_ns() - start) / 1000_000);
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
            image: unsafe {
                    Pixbuf::new(GTK_COLORSPACE_RGB, false, 8, width, height)
                }
                .unwrap(),
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
                        }
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
    fractal: Arc<Box<Fractal>>,
    palette: Palette,
    scale: f64,
    center: Complex64,
    rendering: Option<Mutex<FractalRendering>>,
}

impl FractalWidget {
    fn new() -> Arc<Mutex<FractalWidget>> {
        let area = gtk::DrawingArea::new();
        let result = Arc::new(Mutex::new(FractalWidget {
                                             widget: area,
                                             fractal: Mandelbrot::new(150),
                                             palette: Palette::default(),
                                             scale: 1.2,
                                             center: Complex::from(-0.5),
                                             rendering: None,
                                         }));
        let r2 = result.clone();
        result.lock().unwrap().widget.connect_draw(move |_w, c| {
                                                       r2.lock()
                                                           .unwrap()
                                                           .expose(c)
                                                   });
        result
    }
    fn get_title(&self) -> String {
        format!("{} @ {} ±{:e}", self.fractal, self.center, self.scale)
    }
    fn redraw(&mut self) {
        self.rendering = None;
        self.widget.queue_draw();
    }
    fn zoom(&mut self, z: Complex64, s: f64) {
        self.center = z;
        self.scale *= s;
        self.redraw();
    }
    fn julia(&mut self, z: Complex64) {
        self.fractal = Julia::new(z, 500);
        self.center = Complex64::zero();
        self.scale = 1.2;
        self.redraw();
    }
    fn inc_maxiter(&mut self) {
        self.fractal =
            self.fractal.change_maxiter(&|i| {
                let ten = 10.0_f64;
                i + ten.powi(max(0, ((i / 3) as f64).log10() as i32)) as u32
            });
        println!("Fractal is {}", self.fractal);
        self.redraw();
    }
    fn dec_maxiter(&mut self) {
        self.fractal = self.fractal.change_maxiter(&|i| {
            let ten = 10.0_f64;
            max(i - ten.powi(max(0, ((i / 3) as f64).log10() as i32)) as u32,
                1)
        });
        println!("Fractal is {}", self.fractal);
        self.redraw();
    }
    fn get_xform(&self) -> Transform {
        Transform::new(self.center,
                       self.scale,
                       self.widget.get_allocated_width(),
                       self.widget.get_allocated_height())
    }

    fn expose(&mut self, c: &Context) -> Inhibit {
        //let start = precise_time_ns();
        //println!("redraw ...");
        let (rendered_width, rendered_height) = match self.rendering {
            Some(ref r) => {
                let r = r.lock().unwrap();
                (r.width, r.height)
            }
            _ => (0, 0),
        };
        let width = self.widget.get_allocated_width();
        let height = self.widget.get_allocated_height();
        if rendered_width != width || rendered_height != height {
            self.rendering =
                Some(Mutex::new(FractalRendering::new(width,
                                                      height,
                                                      self.get_xform(),
                                                      self.fractal.clone(),
                                                      self.palette.clone())));
            self.widget.queue_draw();
        } else if let Some(ref r) = self.rendering {
            if let Ok(mut renderer) = r.lock() {
                let done = renderer.do_receive();
                let image = &renderer.image;
                c.set_source_pixbuf(image, 0.0, 0.0);
                c.rectangle(0.0,
                            0.0,
                            image.get_width() as f64,
                            image.get_height() as f64);
                c.fill();
                if !done {
                    self.widget.queue_draw();
                }
            }
        }
        //println!("redraw ... done in {} ms.",
        //         (precise_time_ns() - start) / 1000000);
        Inhibit(true)
    }
}

#[allow(non_upper_case_globals)]
fn main() {
    gtk::init().ok();
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_default_size(800, 600);

    let area = FractalWidget::new();
    if let Ok(a) = area.lock() {
        window.add(&a.widget);
        window.set_title(&a.get_title());
    }
    window.connect_delete_event(|_, _| {
                                    gtk::main_quit();
                                    Inhibit(true)
                                });
    window.connect_key_press_event(|_w, e| {
                                       println!("Key pressed: {:?}",
                                                e.get_keyval());
                                       Inhibit(true)
                                   });
    let a1 = area.clone();
    let w = window.clone();
    window.connect_key_release_event(move |_w, e| {
        println!("{:?}: {}", e.get_event_type(), e.get_keyval());
        match e.get_keyval() {
            key::Escape => gtk::main_quit(),
            key::plus => {
                if let Ok(mut a) = a1.lock() {
                    a.inc_maxiter();
                    w.set_title(&a.get_title());
                }
            }
            key::minus => {
                if let Ok(mut a) = a1.lock() {
                    a.dec_maxiter();
                    w.set_title(&a.get_title());
                }
            }
            key::c => {
                if let Ok(mut a) = a1.lock() {
                    a.palette.cycle();
                    a.redraw();
                }
            }
            key::m => {
                if let Ok(mut a) = a1.lock() {
                    let s = a.scale;
                    a.zoom(Complex::from(-0.5), 1.2 / s);
                    a.fractal = Mandelbrot::new(150);
                    w.set_title(&a.get_title());
                }
            }
            _ => (),
        }
        Inhibit(true)
    });
    let a2 = area.clone();
    let w2 = window.clone();
    window.connect_button_release_event(move |_w, e| {
        if let Ok(mut a) = a2.lock() {
            let (x, y) = e.get_position();
            let z = a.get_xform().xformf(x, y);
            let state = e.get_state();
            println!("Got b button release: {:?} {}", state, state.bits());
            println!("{:?} at {}", e.get_event_type(), z);
            match state {
                BUTTON1_MASK => a.zoom(z, 0.5),
                BUTTON2_MASK => a.julia(z),
                BUTTON3_MASK => a.zoom(z, 2.0),
                _ => (),
            }
            w2.set_title(&a.get_title());
        }
        Inhibit(true)
    });

    window.show_all();
    gtk::main();
}
