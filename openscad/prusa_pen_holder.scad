
nema_17_size = 42.3;
hole_d = 11;
pin_hole = 16;

t = 1.5;
d = 25;

PENCIL = 8;
PEN    = 11.5;

pencil_d = PEN;
pencil_hole_d = 3.75;
pencil_hole_l = 4;

module motor_mount() {
    rotate([90, 0, 90])
    difference() {
        cube([nema_17_size + 2*t, nema_17_size + 2*t, d]);
        translate([t, t, t])
            cube([nema_17_size, nema_17_size, d]);
        translate([nema_17_size/2 + t, nema_17_size/2 + t, 0])
            cylinder(h = d, d = hole_d, $fn = 33);
        translate([nema_17_size/2-pin_hole/2, 2*t, t])
            cube([pin_hole, nema_17_size, d]);
    }
}

module pencil_holder() {
    h = nema_17_size / 1 + t*2;
    tp = pencil_hole_l;
    difference() {
        union() {
            cylinder(h = h, d = pencil_d + 2 * tp, $fn = 33);
            translate([-(pencil_d + 2 * tp)/2, 0, 0])
            #cube([pencil_d + 2 * tp, pencil_d + tp, h]);
        }
        translate([0, 0, -tp])
        cylinder(h = nema_17_size + 2 * tp, d = pencil_d, $fn = 33);
        translate([0, 0, h/2])
        rotate([90, 0, 0])
        cylinder(h = pencil_hole_l + pencil_d, d = pencil_hole_d, $fn = 33);
    }
}

pencil_holder();
translate([-d/2, pencil_d + pencil_hole_l, 0])
motor_mount();