use std::io::prelude::*;
use std::fs::File;
use std::fmt;

const FEED_RATE: f32 = 1000.0;

// TODO: Configurable
const Z0: f32       = 10.0;
const Z_END: f32    = 80.0;
const Z_PLUNGE: f32 = 4.0;

const G_MODE: u32 = 0;

const COMMENTS: bool = true;

pub struct Printer {
    min: (f32, f32),
    _max: (f32, f32),
    pub width:  f32,
    pub height: f32,
    scale: Option<(f32, f32)>,
    commands:   Vec<String>,
    gcode: Vec<Code>,
}

pub enum Code {
    Comment(String),
    Model(String),
    Message(String),
    DisableMotors,
    UnitsMM,
    AbsoluteCoord,
    Move(Option<f32>, Option<f32>, Option<f32>, f32),
    Home,
}

fn coord(c: char, val: &Option<f32>) -> String {
    if let Some(v) = val {
        format!("{}{:.1} ", c, v)
    } else {
        "".to_string()
    }
}

fn move_xy(x: f32, y: f32, speed: f32) -> Code {
    Code::Move(Some(x), Some(y), None, speed)
}

fn move_z(z: f32, speed: f32) -> Code {
    Code::Move(None, None, Some(z), speed)
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Code::Message(m)    => write!(f, "M117 {}", m),
            Code::Comment(c)    => write!(f, "; {}", c),
            Code::Model(m)      => if COMMENTS { write!(f, "M862.3 P {} ; printer model check", m) } else { write!(f, "M862.3 P {}", m) },
            Code::DisableMotors => if COMMENTS { write!(f, "M84 ; disable motors") } else { write!(f, "M84") },
            Code::UnitsMM       => if COMMENTS { write!(f, "G21 ; set units to millimeters") } else { write!(f, "G21") },
            Code::AbsoluteCoord => if COMMENTS { write!(f, "G90 ; use absolute coordinates") } else { write!(f, "G90") },
            Code::Home          => if COMMENTS { write!(f, "G28 W ; home all without mesh bed level") } else { write!(f, "M84") },
            Code::Move(x, y, z, feed) => write!(f, "G{} {}{}{}{}", G_MODE, coord('X', x), coord('Y', y), coord('Z', z), coord('F', &Some(*feed)))
        }
    }
}

impl Printer {
    pub fn new((minx, miny): (f32, f32), (maxx, maxy): (f32, f32)) -> Self {
        Printer {
            min:     (minx, miny),
            _max:    (maxx, maxy),
            width:    maxx - minx,
            height:   maxy - miny,
            scale:    None,
            commands: Vec::new(),
        }.init()
    }

    fn rescale(m: f32, rmin: f32, rmax: f32, tmin: f32, tmax: f32) -> f32 {
        ((m - rmin) / (rmax - rmin)) * (tmax - tmin) + tmin
    }

    pub fn set_scale(&mut self, original_width: f32, original_height: f32) {
        self.scale = Some((original_width, original_height));
    }

    pub fn draw_point(&mut self, xp: f32, yp: f32) {
        let x: f32;
        let y: f32;

        if let Some((ow, oh)) = self.scale {
            x = Self::rescale(xp, 0.0, ow, 0.0, self.width);
            y = Self::rescale(yp, 0.0, oh, 0.0, self.height);
        } else {
            x = xp;
            y = yp;
        }

        self.commands.push(format!("; draw_point({:.1}, {:.1})", xp, yp));
        self.gcode.push(Code::Comment(format!("draw_point({:.1}, {:.1})", xp, yp)));
        
        self.commands.push(format!("G{} X{:.1} Y{:.1} F{:.1}", G_MODE, x, y, FEED_RATE));
        self.gcode.push(move_xy(x, y, FEED_RATE));
        // Pen down for the dot
        self.commands.push(format!("G{} Z{:.1} F100", G_MODE,  Z_PLUNGE));
        self.commands.push(format!("G{} Z{:.1} F100", G_MODE,  Z0));
        self.gcode.push(move_z(Z_PLUNGE, 100.0));
        self.gcode.push(move_z(Z0, 100.0));
        // self.commands.push("G91   ; Switch to relative coordinates".to_owned());
        // self.commands.push(format!("G1 Z-{:.1} F100", Z_PLUNGE));
        // self.commands.push(format!("G1 Z{:.1}  F100", Z_PLUNGE));
        // self.commands.push("G90   ; Switch back to  absolute coordinates".to_owned());
         
        self.commands.push("".to_owned());
    }

    pub fn save(&self, filename: &str) {
        let mut file = File::create(filename).unwrap();
        let mut count = 1;
        // TODO: Configurable
        let skip = 100;
        for cmd in &self.commands {
            // TODO: what about errors?
            _ = file.write_all(cmd.as_bytes());
            _ = file.write_all("\n".as_bytes());

            if count % skip == 0 {
                let percent: f32 = ((count as f32) / (self.commands.len() as f32)) * 100.0;
                _ = file.write_all(format!("M117 {:.1}%\n", percent).as_bytes());
            }

            count += 1;
        }
        _ = file.write_all("; Lift the head up before turning off\n".as_bytes());
        _ = file.write_all("G91   ; Switch to relative coordinates\n".as_bytes());
        _ = file.write_all(format!("G1 Z{:.1}  {:.1}\n", Z_END, FEED_RATE).as_bytes());
        _ = file.write_all("G90   ; Switch back to  absolute coordinates\n".as_bytes());
        _ = file.write_all("M84   ; disable motors\n".as_bytes());
    }

    fn init(mut self) -> Self {
        let (minx, miny) = self.min;

        self.commands.clear();

        // TODO: Configurable
        self.commands.push("M862.3 P \"MK3S\" ; printer model check".to_owned());
        self.commands.push("G21   ; set units to millimeters".to_owned());
        self.commands.push("G90   ; use absolute coordinates".to_owned());
        self.commands.push("G28 W ; home all without mesh bed level".to_owned());
        self.commands.push("".to_owned());
        
        self.commands.push(format!("G1 X{:.1} Y{:.1} Z{:.1} F{:.1}",
                minx,
                miny,
                Z0,
                FEED_RATE));
        self.commands.push("G92 X0 Y0    ; set current position to origin".to_owned());
        self.commands.push("M117 0.0%".to_owned());
        self.commands.push("".to_owned());

        self
    }
}

fn write_code(f: &mut File, c: Code) {
    _ = f.write_all(c.to_string().as_bytes());
    _ = f.write_all("\n".as_bytes());
}

fn gcode_header() -> Vec<Code> {
    vec![
        Code::Model("MK3S".to_owned()),
        Code::UnitsMM,
        Code::AbsoluteCoord,
        Code::Home,
    ]
}

fn gcode_footer() -> Vec<Code> {
    vec![
        Code::Comment("Lift the head up before turning off".to_string()),
        Code::DisableMotors
    ]
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
    fn code_disable_motors() {
        let c: Code = Code::DisableMotors;
        if COMMENTS {
            assert_eq!(c.to_string(), "M84 ; disable motors");
        } else {
            assert_eq!(c.to_string(), "M84");
        }
    }

    #[test]
    fn code_model() {
        let c: Code = Code::Model("MK3S".to_owned());
        if COMMENTS {
            assert_eq!(c.to_string(), "M862.3 P MK3S ; printer model check");
        } else {
            assert_eq!(c.to_string(), "M862.3 P MK3S");
        }
    }

    #[test]
    fn code_units_mm() {
        let c: Code = Code::UnitsMM;
        if COMMENTS {
            assert_eq!(c.to_string(), "G21 ; set units to millimeters");
        } else {
            assert_eq!(c.to_string(), "G21");
        }
    }

    #[test]
    fn code_absolute_coords() {
        let c: Code = Code::AbsoluteCoord;
        if COMMENTS {
            assert_eq!(c.to_string(), "G90 ; use absolute coordinates");
        } else {
            assert_eq!(c.to_string(), "G90");
        }
    }

    #[test]
    fn code_home_all() {
        let c: Code = Code::Home;
        if COMMENTS {
            assert_eq!(c.to_string(), "G28 W ; home all without mesh bed level");
        } else {
            assert_eq!(c.to_string(), "G28 W");
        }
    }

    #[test]
    fn code_move() {
        let c: Code = Code::Move(None, None, None, 0.0);
        assert_eq!(c.to_string(), format!("G{} F0.0 ", G_MODE));
        
        let c: Code = Code::Move(Some(1.0), None, None, 1000.0);
        assert_eq!(c.to_string(), format!("G{} X1.0 F1000.0 ", G_MODE));
        
        let c: Code = Code::Move(Some(1.0), Some(2.0), None, 1000.0);
        assert_eq!(c.to_string(), format!("G{} X1.0 Y2.0 F1000.0 ", G_MODE));
        
        let c: Code = Code::Move(Some(1.0), Some(2.0), Some(3.0), 1000.0);
        assert_eq!(c.to_string(), format!("G{} X1.0 Y2.0 Z3.0 F1000.0 ", G_MODE));
        
        let c: Code = Code::Move(None, None, Some(10.0), 500.0);
        assert_eq!(c.to_string(), format!("G{} Z10.0 F500.0 ", G_MODE));
    }
} 
