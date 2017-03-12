#[derive(Clone, Debug)]
pub struct Palette {
    h0: f32,
    hscale: f32,
}

impl Palette {
    pub fn default() -> Self {
        Palette {
            h0: 0.0,
            hscale: 1.0,
        }
    }

    pub fn cycle(&mut self) {
        if self.h0 * self.hscale > 1.0 {
            self.hscale = -self.hscale;
        } else {
            self.h0 += 0.06 * self.hscale;
        }
    }

    pub fn color(&self, i: f32) -> (u8, u8, u8) {
        if i == 0.0 {
            (0, 0, 0)
        } else {
            let (r, g, b) =
                hsl2rgb(self.h0 + self.hscale * i, 1.0, 0.95 * i + 0.05);
            ((255.0 * r) as u8, (255.0 * g) as u8, (255.0 * b) as u8)
        }
    }
}

fn hsl2rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    let m2 = if l < 0.5 {
        l * (s + 1.0)
    } else {
        l + s - l * s
    };
    let m1 = l * 2.0 - m2;
    let r = hue_to_rgb(m1, m2, h + 1.0 / 3.0);
    let g = hue_to_rgb(m1, m2, h);
    let b = hue_to_rgb(m1, m2, h - 1.0 / 3.0);
    (r, g, b)
}

fn hue_to_rgb(m1: f32, m2: f32, h: f32) -> f32 {
    let h = if h < 0.0 {
        h + 1.0
    } else if h > 1.0 {
        h - 1.0
    } else {
        h
    };
    if h * 6.0 < 1.0 {
        m1 + (m2 - m1) * h * 6.0
    } else if h * 2.0 < 1.0 {
        m2
    } else if h * 3.0 < 2.0 {
        m1 + (m2 - m1) * (2.0 / 3.0 - h) * 6.0
    } else {
        m1
    }
}
