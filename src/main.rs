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
use gtk::widgets::Widget;
use num::complex::Complex64;
use num::complex::Complex;
use time::precise_time_ns;

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

fn redraw(w : Widget, c : Context) -> Inhibit {
    let width = w.get_allocated_width();
    let height = w.get_allocated_height();
    let image = unsafe { gdk::Pixbuf::new(ColorSpace::RGB, false, 8, width, height) }.unwrap();

    println!("Should render {} x {} ...", width, height);
    let start = precise_time_ns();
    let scale = 2.5 as f64;
    let maxiter = 150 as u8;

    let s = scale / height as f64;
    let xform = |x : i32, y : i32| {
       return Complex64 {re: (x - 2*width/3) as f64, im: (y - height/2) as f64}.scale(s);
    };

    let zero = Complex {re: 0.0, im: 0.0};
    //let c = Complex {re: -0.75, im: 0.12};

    let n_channels = image.get_n_channels();
    let rowstride = image.get_rowstride();
    let data = unsafe { image.get_pixels() };

    for y in 0..height {
        for x in 0..width {
            let pos = (y * rowstride + x * n_channels) as usize;
            let i = julia(zero, xform(x, y), maxiter);
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

struct FractalWidget {
    widget: gtk::DrawingArea
}

impl FractalWidget {
    fn new() -> FractalWidget {
        let area = gtk::DrawingArea::new().unwrap();
        area.connect_draw(redraw);
        FractalWidget {
            widget: area
        }
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
