extern crate cairo;
extern crate gdk;
extern crate gtk;
extern crate num;
extern crate time;

use cairo::Context;
use gdk::ColorSpace;
use gdk::Pixbuf;
use gdk::cairo_interaction::ContextExt;
use gtk::signal::Inhibit;
use gtk::signal::WidgetSignals;
use gtk::traits::ContainerTrait;
use gtk::traits::WidgetTrait;
use gtk::traits::WindowTrait;
use num::complex::Complex64;
use num::complex::Complex;
use time::precise_time_ns;
use std::sync::Arc;
use std::cmp::min;

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

struct FractalWidget {
    widget: gtk::DrawingArea,
    maxiter: u8,
    scale: f64,
    center: Complex64
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
}

impl FractalWidget {
    fn new() -> Arc<FractalWidget> {
        let area = gtk::DrawingArea::new().unwrap();
        let result = Arc::new(FractalWidget {
            widget: area,
            maxiter: 150,
            scale: 1.2,
            center: Complex{re: -0.5, im: 0.0}
        });
        let r2 = result.clone();
        result.widget.connect_draw(move |_w, c| r2.redraw(c));
        result
    }

    fn get_xform(&self) -> Transform {
        Transform::new(self.center, self.scale,
                       self.widget.get_allocated_width(),
                       self.widget.get_allocated_height())
    }

    fn redraw(&self, c : Context) -> Inhibit {
        let ref w = self.widget;
        let width = w.get_allocated_width();
        let height = w.get_allocated_height();
        let image = unsafe { gdk::Pixbuf::new(ColorSpace::RGB, false, 8, width, height) }.unwrap();

        println!("Should render {} x {} ...", width, height);
        let start = precise_time_ns();
        let zero = Complex {re: 0.0, im: 0.0};
        let xform = self.get_xform();
        //let c = Complex {re: -0.75, im: 0.12};

        let n_channels = image.get_n_channels();
        let rowstride = image.get_rowstride();
        let data = unsafe { image.get_pixels() };

        for y in 0..height {
            for x in 0..width {
                let pos = (y * rowstride + x * n_channels) as usize;
                let i = julia(zero, xform.xform(x, y), self.maxiter);
                data[pos] = i as u8;
                data[pos + 1] = i as u8;
                data[pos + 2] = i as u8;
            }
        }
        println!("Should render ... done in {} ms.",
                 (precise_time_ns() - start) / 1000000);

        c.set_source_pixbuf(&image, 0.0, 0.0);
        c.rectangle(0.0, 0.0, image.get_width() as f64, image.get_height() as f64);
        c.fill();
        Inhibit(true)
    }
}

fn main() {
    gtk::init().ok();
    let window = gtk::Window::new(gtk::WindowType::TopLevel).unwrap();
    window.set_title("KratsFract");
    window.set_default_size(800, 600);
    window.set_window_position(gtk::WindowPosition::Center);

    let area = FractalWidget::new();
    window.add(&area.widget);
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(true)
    });

    window.show_all();
    gtk::main();
}
