
use std::fmt;
use std::fs::File;
use std::io::prelude::*;

const G_MODE:  u32 = 0;
const Z_RESET: f32 = 80.0;

const SPEED: f32 = 200.0;
const MAX_FEED: f32 = 1000.0;

#[derive(Debug, Clone)]
pub struct Source {
    code:    &'static str,
    comment: Option<&'static str>,
}

#[derive(Debug, Copy, Clone)]
pub struct Point {
    x: Option<f32>,
    y: Option<f32>,
    z: Option<f32>,
}

#[derive(Debug, Clone)]
pub enum Code {
    Comment(String),
    Model(String),
    Message(String),
    Move(Point, f32),
    Raw(Source),
    NOP
}

#[derive(Debug, Clone)]
pub struct PrinterConfig {
    pub model: Option<Code>,
    pub min: (f32, f32),
    pub max: (f32, f32),
    pub scale: Option<(f32, f32)>,
    pub z0: f32,
    pub z_plunge: f32,
    pub move_speed: f32,
    pub plunge_speed: f32,
    pub retract_speed: f32,
}

pub struct Printer {
    config: PrinterConfig,
    code: Vec<Code>,
    pub width:  f32,
    pub height: f32,
}

macro_rules! raw{
    ($a: expr, $b: expr) => {
        {
            Code::Raw(Source {code: $a, comment: Some($b)})
        }
    };
    ($a: expr) => {
        {
            Code::Raw(Source {code: $a, comment: None})
        }
    }
}

const HOME: Code       = raw!("G28 W",     "Home all without mesh bed level");
const UNITS_MM: Code   = raw!("G21",       "Set units to millimeters");
const ABS_COORD: Code  = raw!("G90",       "Use absolute coordinates");
const SET_ORIGIN: Code = raw!("G92 X0 Y0", "Set current position to origin");
const OFF: Code        = raw!("M84",       "Disable motors");

fn rescale(m: f32, rmin: f32, rmax: f32, tmin: f32, tmax: f32) -> f32 {
    ((m - rmin) / (rmax - rmin)) * (tmax - tmin) + tmin
}

fn render_coord(axis: char, v: Option<f32>) -> String {
    if let Some(value) = v {
        format!("{}{:.1}", axis, value)
    } else {
        "".to_string()
    }
}

fn write_code(f: &mut File, c: Code) {
    // TODO: What to do in case of error
    _ = f.write_all(c.to_string().as_bytes());
    _ = f.write_all("\n".as_bytes());
}

fn render_move(point: &Point, feed: &f32) -> String {
    let point_str = point.to_string();

    if point_str.len() == 0 {
        Code::Comment("[WARNING] Move without coordinates!".to_string()).to_string()
    } else {
        format!("G{} {} F{:.1}", G_MODE, point_str, feed)
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let x = render_coord('X', self.x);
        let y = render_coord('Y', self.y);
        let z = render_coord('Z', self.z);

        let x_space = if x.len() > 0 && (y.len() > 0 || z.len() > 0) {
            " "
        } else {
            ""
        };
        
        let y_space = if y.len() > 0 && z.len() > 0 {
            " "
        } else {
            ""
        };

        write!(f, "{}{}{}{}{}", x, x_space, y, y_space, z)
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(comment) = self.comment {
            write!(f, "{} ; {}", self.code, comment)
        } else {
            write!(f, "{}", self.code)
        }
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Code::Comment(c) => write!(f, "; {}", c),
            Code::Model(m)   => write!(f, "M862.3 P \"{}\" ; printer model check", m),
            Code::Message(m) => write!(f, "M117 {}", m),
            Code::Move(p, s) => write!(f, "{}", render_move(p, s)),
            Code::Raw(src)   => write!(f, "{}", src.to_string()),
            Code::NOP        => write!(f, ""),
        }
    }
}


impl Printer {

    pub fn new(config: PrinterConfig) -> Self {
        Printer {
            config: config.clone(),
            code:   Vec::new(),
            width:  config.max.0 - config.min.0,
            height: config.max.1 - config.min.1,
        }
    }

    pub fn draw_point(&mut self, xp: f32, yp: f32) {
        let x: f32;
        let y: f32;

        if let Some((ow, oh)) = self.config.scale {
            x = rescale(xp, 0.0, ow, 0.0, self.width);
            y = rescale(yp, 0.0, oh, 0.0, self.height);
        } else {
            x = xp;
            y = yp;
        }

        self.code.push(Code::Comment(format!("draw_point({:.1}, {:.1})", xp, yp)));
        self.code.push(Code::Move(Point{x: Some(x), y: Some(y), z: None},                       self.config.move_speed));
        self.code.push(Code::Move(Point{x: None,    y: None,    z: Some(self.config.z_plunge)}, self.config.plunge_speed));
        self.code.push(Code::Move(Point{x: None,    y: None,    z: Some(self.config.z0)},       self.config.retract_speed));
        self.code.push(Code::NOP);
    }

    fn calc_total_time(&self) -> f32 {
        let mut pos_x = self.config.min.0;
        let mut pos_y = self.config.min.1;
        let mut pos_z = self.config.z0;
        let mut total_dist = 0.0;
        
        for c in &self.code {

            if let Code::Move(p, f) = c {
                let speed_scale = f / MAX_FEED;

                let dx = if let Some(px) = p.x {
                    pos_x - px
                } else {
                    0.0
                };

                let dy = if let Some(py) = p.y {
                    pos_y - py
                } else {
                    0.0
                };

                let dz = if let Some(pz) = p.z {
                    pos_z - pz
                } else {
                    0.0
                };

                total_dist += dx / speed_scale;
                total_dist += dy / speed_scale;
                total_dist += dz / speed_scale;

                pos_x += dx;
                pos_y += dy;
                pos_z += dz;
            }

        }

        total_dist / SPEED // Should be approx. the number of seconds this will take to print
    }

    pub fn save(&self, filename: &str) {
        let mut file = File::create(filename).unwrap();
        let mut header: Vec<Code> = Vec::new();
        let mut footer: Vec<Code> = Vec::new();

        // header
        header.push(Code::Comment("Start of generated code".to_string()));
        if let Some(model) = &self.config.model {
            header.push(model.clone());
        }
        header.push(UNITS_MM);
        header.push(ABS_COORD);
        header.push(HOME);
        header.push(Code::NOP);
        
        header.push(Code::Move(Point{
            x: Some(self.config.min.0),
            y: Some(self.config.min.1),
            z: Some(self.config.z0)},
            self.config.move_speed));
        header.push(SET_ORIGIN);
        header.push(Code::Message("0.0%".to_string()));
        header.push(Code::NOP);
        
        // footer
        footer.push(Code::Comment("Lift the head up before turning off".to_string()));
        footer.push(Code::Move(Point{
            x: None, y: None, z: Some(Z_RESET)},
            self.config.move_speed));
        footer.push(OFF);
        header.push(Code::NOP);

        for c in header {
            write_code(&mut file, c);
        }

        let mut count = 1;
        // TODO: Configurable
        let skip = 100;
        let total_time = Self::calc_total_time(self) as u32;
        for c in &self.code {
            write_code(&mut file, c.clone());

            if count % skip == 0 {
                let percent: f32 = (count as f32) / (self.code.len() as f32);
                let total_seconds = ((1.0 - percent) * total_time as f32) as u32;
                let hours = total_seconds / 3600;
                let minutes = (total_seconds % 3600) / 60;
                let seconds = total_seconds % 60;

                write_code(&mut file, Code::Message(format!("{:.1}% R{:02}:{:02}:{:02}", percent * 100.0, hours, minutes, seconds)));
            }

            count = count + 1;
        }
        
        for c in footer {
            write_code(&mut file, c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_mesage() {
        let c: Code = Code::Message("Hello".to_owned());
        assert_eq!(c.to_string(), "M117 Hello");
        
        let c: Code = Code::Message(format!("{:.1}%", 50.3));
        assert_eq!(c.to_string(), "M117 50.3%");
    }
    
    #[test]
    fn code_model() {
        let c: Code = Code::Model("MK3S".to_owned());
        assert_eq!(c.to_string(), "M862.3 P \"MK3S\" ; printer model check");
    }

    #[test]
    fn code_move() {
        let p = Point{x: None, y: None, z: None};
        let c: Code = Code::Move(p, 1000.0);
        assert_eq!(c.to_string(), "; [WARNING] Move without coordinates!");
        
        let p = Point{x: Some(0.0), y: None, z: None};
        let c: Code = Code::Move(p, 1000.0);
        assert_eq!(c.to_string(), format!("G{} X0.0 F1000.0", G_MODE));
        
        let p = Point{x: Some(0.0), y: Some(1.0), z: None};
        let c: Code = Code::Move(p, 1000.0);
        assert_eq!(c.to_string(), format!("G{} X0.0 Y1.0 F1000.0", G_MODE));

        let p = Point{x: Some(0.0), y: Some(1.0), z: Some(2.0)};
        let c: Code = Code::Move(p, 1000.0);
        assert_eq!(c.to_string(), format!("G{} X0.0 Y1.0 Z2.0 F1000.0", G_MODE));

        let p = Point{x: Some(0.0), y: None, z: Some(2.0)};
        let c: Code = Code::Move(p, 1000.0);
        assert_eq!(c.to_string(), format!("G{} X0.0 Z2.0 F1000.0", G_MODE));
        
        let p = Point{x: None, y: None, z: Some(2.0)};
        let c: Code = Code::Move(p, 1000.0);
        assert_eq!(c.to_string(), format!("G{} Z2.0 F1000.0", G_MODE));

        let p = Point{x: None, y: Some(1.0), z: None};
        let c: Code = Code::Move(p, 1000.0);
        assert_eq!(c.to_string(), format!("G{} Y1.0 F1000.0", G_MODE));
    }

    #[test]
    fn simple_example() {
        let config: PrinterConfig = PrinterConfig {
            model: Some(Code::Model("MK3S".to_string())),
            min:   (50.0,  35.0),
            max:   (254.0, 212.0),
            scale: None,
            z0:       6.5,
            z_plunge: 4.0,
            move_speed:    1000.0,
            plunge_speed:  500.0,
            retract_speed: 800.0
        };
        let mut printer = Printer::new(config);
        printer.draw_point(50.0, 50.0);
        printer.save("simple_example.gcode");
    }
    
    #[test]
    fn progress_example() {
        let config: PrinterConfig = PrinterConfig {
            model: Some(Code::Model("MK3S".to_string())),
            min:   (50.0,  35.0),
            max:   (254.0, 212.0),
            scale: None,
            z0:       6.5,
            z_plunge: 4.0,
            move_speed:    1000.0,
            plunge_speed:  500.0,
            retract_speed: 800.0
        };
        let mut printer = Printer::new(config);
        for _i in 0..10000 {
            printer.draw_point(50.0, 50.0);
        }
        printer.save("progress.gcode");
    }
} 
