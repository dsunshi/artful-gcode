

pen_shaft_d = 11.5;
pen_cap_d   = 9.5;

pen_cap_l   = 6.5;
pen_shaft_l = 8.5;

total_d = 20;

height = pen_cap_l + pen_shaft_l;
ROUND  = 33;

difference() {
    cylinder(h = height,      d = total_d,     $fn = ROUND);
    cylinder(h = pen_cap_l,   d = pen_cap_d,   $fn = ROUND);
    translate([0, 0, pen_cap_l])
    cylinder(h = pen_shaft_l, d = pen_shaft_d, $fn = ROUND);
}