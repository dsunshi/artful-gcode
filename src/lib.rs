
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::cmp;

const G_MODE:  u32 = 0;
const Z_RESET: f32 = 80.0;

const SPEED: f32 = 10.0;
const _MAX_FEED: f32 = 1000.0;

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

macro_rules! xy{
    ($a: expr, $b: expr, $c: expr) => {
        {
            Code::Move(Point{x: Some($a), y: Some($b), z: None}, $c)
        }
    }
}

macro_rules! z{
    ($a: expr, $b: expr) => {
        {
            Code::Move(Point{x: None, y: None, z: Some($a)}, $b)
        }
    }
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

fn delta_point(a: &Point, b: &Point) -> f32 {
    let diff = |a: f32, b: f32| -> f32 { a - b };

    let delta_x = a.x.zip(b.x).map(|(x, y)| diff(x, y)).unwrap_or(0.0);
    let delta_y = a.y.zip(b.y).map(|(x, y)| diff(x, y)).unwrap_or(0.0);
    let delta_z = a.z.zip(b.z).map(|(x, y)| diff(x, y)).unwrap_or(0.0);

    ((delta_x).powf(2.0) + (delta_y).powf(2.0) + (delta_z).powf(2.0)).sqrt()
}

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
        // -> (x, y)
        self.code.push(xy!(x, y, self.config.move_speed));
        // pen down
        self.code.push(z!(self.config.z_plunge, self.config.plunge_speed));
        // pen up
        self.code.push(z!(self.config.z0, self.config.retract_speed));
        self.code.push(Code::NOP);
    }

    fn total_dist(&self) -> f32 {
        let mut total_dist = 0.0;

        let mut curr_point = Point {
            x: Some(0.0),
            y: Some(0.0),
            z: Some(self.config.z0) };

        for c in &self.code {

            if let Code::Move(p, _) = c {
                total_dist  += delta_point(&curr_point, &p);

                curr_point.x = p.x.or(curr_point.x);
                curr_point.y = p.y.or(curr_point.y);
                curr_point.z = p.z.or(curr_point.z);
            }

        }

        total_dist
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

        // Z first so we don't scrape the print area
        header.push(z!(self.config.z0, self.config.move_speed));
        header.push(xy!(self.config.min.0, self.config.min.1, self.config.move_speed));
        header.push(SET_ORIGIN);
        header.push(Code::Message("0.0%".to_string()));
        header.push(Code::NOP);

        // footer
        footer.push(Code::Comment("Lift the head up before turning off".to_string()));
        footer.push(z!(Z_RESET, self.config.move_speed));
        footer.push(OFF);
        footer.push(Code::NOP);

        for c in header {
            write_code(&mut file, c);
        }

        let mut count = 1;
        let skip = cmp::max(((self.code.len() as f32) * 0.015) as u32, 5); // 5 number of commands
                                                                           // in draw_point
        let total_time = (Self::total_dist(self) / SPEED) as u32;
        for c in &self.code {
            write_code(&mut file, c.clone());

            if count % skip == 0 {
                let percent: f32 = (count as f32) / (self.code.len() as f32);
                let total_seconds = ((1.0 - percent) * total_time as f32) as u32;
                let hours = total_seconds / 3600;
                let minutes = (total_seconds % 3600) / 60;
                let seconds = total_seconds % 60;

                write_code(&mut file, Code::Message(
                        format!("{:.1}% R{:02}:{:02}:{:02}", percent * 100.0, hours, minutes, seconds)));
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

    fn test_config() -> PrinterConfig {
        PrinterConfig {
            model: Some(Code::Model("MK3S".to_string())),
            min:   (50.0,  35.0),
            max:   (254.0, 212.0),
            scale: None,
            z0:       6.5,
            z_plunge: 4.0,
            move_speed:    1000.0,
            plunge_speed:  500.0,
            retract_speed: 800.0
        }
    }

    fn assert_within(a: f32, b: f32, n: f32) {
        if (a - b).abs() >= n {
            panic!("The difference between {} and {} is more than {}!", a, b, n);
        }
    }

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
        let mut printer = Printer::new(test_config());
        printer.draw_point(50.0, 50.0);
        printer.save("simple_example.gcode");
    }

    #[test]
    fn progress_example() {
        let mut printer = Printer::new(test_config());
        for _i in 0..10000 {
            printer.draw_point(50.0, 50.0);
        }
        printer.save("progress.gcode");
    }

    #[test]
    fn speed_example() {
        let mut printer = Printer::new(test_config());
        for i in 0..1000 {
            printer.draw_point(i as f32, i as f32);
        }
        printer.save("speed.gcode");
    }

    #[test]
    fn dist_test() {
        let mut printer = Printer::new(test_config());
        printer.draw_point(50.0, 49.0);
        printer.draw_point(29.0, 29.0);
        // 99.0 is the distance from (0, 0) to (50, 49) to (29, 29)
        // Each point is drawn by going down then back up, i.e.:
        //  2 * (z0 - z_plunge)
        // We drew two points, so the total formula is:
        let expected = 99.0 + (2.0 * (2.0 * (6.5 - 4.0)));
        let actual = printer.total_dist();
        assert_within(actual, expected, 0.01);
    }
}

