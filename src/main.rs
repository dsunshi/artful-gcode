
use std::io::prelude::*;
use std::fs::File;
use std::fmt;

const G_MODE:  u32 = 0;
const Z_RESET: f32 = 80.0;

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
}

#[derive(Debug, Clone)]
pub struct PrinterConfig {
    model: Option<Code>,
    min: (f32, f32),
    max: (f32, f32),
    scale: Option<(f32, f32)>,
    z0: f32,
    z_plunge: f32,
    move_speed: f32,
    plunge_speed: f32,
    retract_speed: f32,
}

pub struct Printer {
    config: PrinterConfig,
    code: Vec<Code>,
    pub width:  f32,
    pub height: f32,
}

const HOME: Code       = Code::Raw(Source {code: "G28 W",     comment: Some("Home all without mesh bed level")});
const UNITS_MM: Code   = Code::Raw(Source {code: "G21",       comment: Some("Set units to millimeters")});
const ABS_COORD: Code  = Code::Raw(Source {code: "G90",       comment: Some("Use absolute coordinates")});
const SET_ORIGIN: Code = Code::Raw(Source {code: "G92 X0 Y0", comment: Some("Set current position to origin")});
const OFF: Code        = Code::Raw(Source {code: "M84",       comment: Some("Disable motors")});

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

fn render_move(point: &Point, feed: &f32) -> String {
    let point_str = point.to_string();

    if point_str.len() == 0 {
        Code::Comment("[WARNING] Move without coordinates!".to_string()).to_string()
    } else {
        format!("G{} {} F{:.1}", G_MODE, point_str, feed)
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
            Code::Model(m)   => write!(f, "M862.3 P {} ; printer model check", m),
            Code::Message(m) => write!(f, "M117 {}", m),
            Code::Move(p, s) => write!(f, "{}", render_move(p, s)),
            Code::Raw(src)   => write!(f, "{}", src.to_string()),
        }
    }
}

impl Code {
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
        self.code.push(Code::Move(Point{x: Some(x), y: Some(y), z: None}, self.config.move_speed));
        self.code.push(Code::Move(Point{x: None, y: None, z: Some(self.config.z_plunge)}, self.config.plunge_speed));
        self.code.push(Code::Move(Point{x: None, y: None, z: Some(self.config.z0)}, self.config.retract_speed));
    }

    pub fn save(&self, filename: &str) {
        let mut file = File::create(filename).unwrap();
        let mut commands: Vec<Code> = Vec::new();

        commands.push(Code::Comment("Start of generated code".to_string()));
        if let Some(model) = &self.config.model {
            commands.push(model.clone());
        }
        commands.push(UNITS_MM);
        commands.push(ABS_COORD);
        commands.push(HOME);
        
        commands.push(Code::Move(Point{
            x: Some(self.config.min.0),
            y: Some(self.config.min.1),
            z: Some(self.config.z0)},
            self.config.move_speed));
        commands.push(SET_ORIGIN);
        commands.push(Code::Message("0.0%".to_string()));


        // let mut count = 1;
        // // TODO: Configurable
        // let skip = 100;
        // for cmd in &self.commands {
        //     // TODO: what about errors?
        //     _ = file.write_all(cmd.as_bytes());
        //     _ = file.write_all("\n".as_bytes());
        //
        //     if count % skip == 0 {
        //         let percent: f32 = ((count as f32) / (self.commands.len() as f32)) * 100.0;
        //         _ = file.write_all(format!("M117 {:.1}%\n", percent).as_bytes());
        //     }
        //
        //     count += 1;
        // }
        commands.push(Code::Comment("Lift the head up before turning off".to_string()));
        commands.push(Code::Move(Point{
            x: None, y: None, z: Some(Z_RESET)},
            self.config.move_speed));
        commands.push(OFF);
    }


}

fn main() {
    let xs = vec![ Point{x: None, y: None, z: None},
                    Point{x: Some(0.0), y: None, z: None},
                    Point{x: Some(0.0), y: Some(1.0), z: None},
                    Point{x: Some(0.0), y: Some(1.0), z: Some(2.0)},
                    Point{x: Some(0.0), y: None, z: Some(2.0)},
                    Point{x: None, y: None, z: Some(2.0)},
                    Point{x: None, y: Some(1.0), z: None} ];

    for p in xs {
        let m: Code = Code::Move(p, 1000.0);
        println!("Point: |{}|", p.to_string());
        println!("Move: |{}|", m.to_string());
        println!("");
    }

    let config: PrinterConfig = PrinterConfig {
        model: Some(Code::Model("MK3S".to_string())),
        min:   (0.0, 0.0),
        max:   (0.0, 0.0),
        scale: None,
        z0:       6.5,
        z_plunge: 4.0,
        move_speed:    1000.0,
        plunge_speed:  400.0,
        retract_speed: 800.0
    };
}
