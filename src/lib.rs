use std::io::prelude::*;
use std::fs::File;

const FEED_RATE: f32 = 1000.0;

const Z0: f32       = 10.0;
const Z_END: f32    = 80.0;
const Z_PLUNGE: f32 = 4.0;

const G_MODE: u32 = 0;

pub struct Printer {
    min: (f32, f32),
    _max: (f32, f32),
    pub width:  f32,
    pub height: f32,
    mode : PrintMode,
    scale: Option<(f32, f32)>,
    commands:   Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PrintMode {
    LINES,
    DOTS,
}

impl Printer {
    pub fn new( (minx, miny): (f32, f32),
                (maxx, maxy): (f32, f32),
                mode: PrintMode) -> Self {
        Printer {
            min:     (minx, miny),
            _max:     (maxx, maxy),
            width:    maxx - minx,
            height:   maxy - miny,
            mode:     mode,
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

    pub fn goto(&mut self, xp: f32, yp: f32) {
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
        // self.commands.push(format!("M117 ({:.1}, {:.1}) -> ({:.1}, {:.1})", xp, yp, x, y));
        if self.mode == PrintMode::DOTS {
            self.commands.push(format!("G{} X{:.1} Y{:.1} F{:.1}", G_MODE, x, y, FEED_RATE));
            // Pen down for the dot
            self.commands.push(format!("G{} Z{:.1} F100", G_MODE,  Z_PLUNGE));
            self.commands.push(format!("G{} Z{:.1} F100", G_MODE,  Z0));
            // self.commands.push("G91   ; Switch to relative coordinates".to_owned());
            // self.commands.push(format!("G1 Z-{:.1} F100", Z_PLUNGE));
            // self.commands.push(format!("G1 Z{:.1}  F100", Z_PLUNGE));
            // self.commands.push("G90   ; Switch back to  absolute coordinates".to_owned());
        } else if self.mode == PrintMode::LINES {
            self.commands.push(format!("G{} X{:.1} Y{:.1} F{:.1}", G_MODE, x, y, FEED_RATE));
        }
        else {
        }
        self.commands.push("".to_owned());
    }

    pub fn save(&self, filename: &str) {
        let mut file = File::create(filename).unwrap();
        // TODO: what about errors?
        let mut count = 1;
        let skip = 100;
        for cmd in &self.commands {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
    }
}
