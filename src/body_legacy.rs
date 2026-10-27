//! Original silhouette kept only for records created before the SVG map.
use adw::prelude::*;
pub fn draw(widget: &gtk::DrawingArea, cr: &gtk::cairo::Context, back: bool) {
    let ink = widget.color();
    cr.set_source_rgba(
        ink.red() as f64,
        ink.green() as f64,
        ink.blue() as f64,
        0.13,
    );
    cr.save().unwrap();
    cr.translate(0.5, 0.10);
    cr.scale(0.065, 0.06);
    cr.arc(0., 0., 1., 0., std::f64::consts::TAU);
    let _ = cr.fill();
    cr.restore().unwrap();
    let outline = [
        (0.46, 0.17),
        (0.54, 0.17),
        (0.55, 0.2),
        (0.65, 0.23),
        (0.72, 0.43),
        (0.75, 0.55),
        (0.70, 0.57),
        (0.64, 0.44),
        (0.59, 0.32),
        (0.58, 0.49),
        (0.61, 0.59),
        (0.58, 0.92),
        (0.61, 0.95),
        (0.51, 0.95),
        (0.50, 0.64),
        (0.49, 0.95),
        (0.39, 0.95),
        (0.42, 0.92),
        (0.39, 0.59),
        (0.42, 0.49),
        (0.41, 0.32),
        (0.36, 0.44),
        (0.30, 0.57),
        (0.25, 0.55),
        (0.28, 0.43),
        (0.35, 0.23),
        (0.45, 0.2),
    ];
    cr.move_to(outline[0].0, outline[0].1);
    for (x, y) in &outline[1..] {
        cr.line_to(*x, *y);
    }
    cr.close_path();
    let _ = cr.fill();
    cr.set_source_rgba(ink.red() as f64, ink.green() as f64, ink.blue() as f64, 0.4);
    cr.set_line_width(0.003);
    cr.move_to(0.5, 0.23);
    cr.line_to(0.5, 0.49);
    let _ = cr.stroke();
    // Original anatomical guides: scapulae on the back, chest on the front.
    for mirror in [false, true] {
        let x = |value: f64| if mirror { 1.0 - value } else { value };
        cr.move_to(x(0.44), 0.24);
        if back {
            cr.curve_to(x(0.40), 0.27, x(0.43), 0.34, x(0.47), 0.37);
            cr.line_to(x(0.48), 0.28);
        } else {
            cr.curve_to(x(0.42), 0.28, x(0.43), 0.32, x(0.48), 0.32);
        }
        let _ = cr.stroke();
        cr.move_to(x(0.43), 0.51);
        cr.curve_to(x(0.45), 0.54, x(0.47), 0.54, x(0.49), 0.55);
        let _ = cr.stroke();
        cr.move_to(x(0.43), 0.74);
        cr.line_to(x(0.47), 0.74);
        let _ = cr.stroke();
    }
}
