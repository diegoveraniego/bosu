use adw::prelude::*;
use gtk::{Box, Button, DrawingArea, DropDown, Label, Orientation, glib};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, rc::Rc};

const KINDS: [&str; 6] = [
    "Dolor",
    "Tensión",
    "Cansancio",
    "Irritado",
    "Trabajado",
    "Fuerte",
];
const COLORS: [(f64, f64, f64); 6] = [
    (0.9, 0.2, 0.2),
    (0.95, 0.65, 0.1),
    (0.5, 0.35, 0.85),
    (0.9, 0.3, 0.6),
    (0.15, 0.6, 0.9),
    (0.2, 0.7, 0.4),
];
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Point {
    x: f64,
    y: f64,
    back: bool,
    kind: usize,
}
#[derive(Clone, Serialize, Deserialize)]
struct Snapshot {
    date: String,
    points: Vec<Point>,
}
fn path() -> std::path::PathBuf {
    glib::user_data_dir().join("bosu_body.json")
}
fn persist(history: &[Snapshot]) -> Result<(), String> {
    std::fs::create_dir_all(glib::user_data_dir()).map_err(|e| e.to_string())?;
    let temp = path().with_extension("json.tmp");
    std::fs::write(
        &temp,
        serde_json::to_vec_pretty(history).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(temp, path()).map_err(|e| e.to_string())
}
fn hit(points: &[Point], x: f64, y: f64, back: bool) -> Option<usize> {
    points
        .iter()
        .rposition(|p| p.back == back && ((p.x - x).powi(2) + (p.y - y).powi(2)).sqrt() < 0.045)
}

pub fn build() -> gtk::ScrolledWindow {
    let root = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(20)
        .margin_end(20)
        .build();
    root.append(
        &Label::builder()
            .label("Mi cuerpo")
            .css_classes(["title-1"])
            .build(),
    );
    root.append(&Label::builder().label("Elige una sensación y toca el cuerpo para crear un punto. Arrastra para moverlo; toca un punto para cambiar su color. Clic derecho para eliminar.").wrap(true).build());
    let side = DropDown::from_strings(&["Frente", "Espalda"]);
    let kind = DropDown::from_strings(&KINDS);
    let views = Box::builder()
        .orientation(Orientation::Horizontal)
        .halign(gtk::Align::Center)
        .css_classes(["linked"])
        .build();
    let front = gtk::ToggleButton::with_label("Frente");
    let back = gtk::ToggleButton::with_label("Espalda");
    back.set_group(Some(&front));
    front.set_active(true);
    front.connect_toggled(glib::clone!(
        #[strong]
        side,
        move |button| {
            if button.is_active() {
                side.set_selected(0);
            }
        }
    ));
    back.connect_toggled(glib::clone!(
        #[strong]
        side,
        move |button| {
            if button.is_active() {
                side.set_selected(1);
            }
        }
    ));
    views.append(&front);
    views.append(&back);
    root.append(&views);
    root.append(&kind);
    let legend = Label::builder().use_markup(true).wrap(true).label("<span foreground='#e63333'>● Dolor</span>   <span foreground='#f2a619'>● Tensión</span>   <span foreground='#8059d9'>● Cansancio</span>\n<span foreground='#e64d99'>● Irritado</span>   <span foreground='#2699e6'>● Trabajado</span>   <span foreground='#33b366'>● Fuerte</span>").build();
    root.append(&legend);
    let (initial, load_error) = match std::fs::read(path()) {
        Ok(bytes) => match serde_json::from_slice::<Vec<Snapshot>>(&bytes) {
            Ok(h) => (h, None),
            Err(e) => (vec![], Some(e.to_string())),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (vec![], None),
        Err(e) => (vec![], Some(e.to_string())),
    };
    let history = Rc::new(RefCell::new(initial));
    let points = Rc::new(RefCell::new(
        history
            .borrow()
            .last()
            .map(|s| s.points.clone())
            .unwrap_or_default(),
    ));
    let area = DrawingArea::builder()
        .content_width(300)
        .content_height(430)
        .halign(gtk::Align::Center)
        .tooltip_text("Mapa corporal: frente y espalda se guardan juntos")
        .build();
    area.update_property(&[gtk::accessible::Property::Label(
        "Mapa corporal interactivo",
    )]);
    area.set_draw_func(glib::clone!(
        #[strong]
        points,
        #[weak]
        side,
        move |widget, cr, w, h| {
            cr.scale(w as f64, h as f64);
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
                if side.selected() == 1 {
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
            for p in points
                .borrow()
                .iter()
                .filter(|p| p.back == (side.selected() == 1))
            {
                let color = COLORS[p.kind.min(COLORS.len() - 1)];
                cr.set_source_rgb(color.0, color.1, color.2);
                cr.save().unwrap();
                cr.translate(p.x, p.y);
                cr.scale(0.025, 0.018);
                cr.arc(0., 0., 1., 0., std::f64::consts::TAU);
                let _ = cr.fill_preserve();
                cr.set_source_rgb(1., 1., 1.);
                cr.set_line_width(0.15);
                let _ = cr.stroke();
                cr.restore().unwrap();
            }
        }
    ));
    side.connect_selected_notify(glib::clone!(
        #[weak]
        area,
        move |_| area.queue_draw()
    ));
    let drag = gtk::GestureDrag::new();
    let active = Rc::new(RefCell::new(None::<(usize, f64, f64)>));
    drag.connect_drag_begin(glib::clone!(
        #[strong]
        points,
        #[strong]
        active,
        #[weak]
        area,
        #[weak]
        side,
        #[weak]
        kind,
        move |_, x, y| {
            let x = x / area.width() as f64;
            let y = y / area.height() as f64;
            let back = side.selected() == 1;
            let mut ps = points.borrow_mut();
            let index = hit(&ps, x, y, back).unwrap_or_else(|| {
                ps.push(Point {
                    x,
                    y,
                    back,
                    kind: kind.selected() as usize,
                });
                ps.len() - 1
            });
            ps[index].kind = kind.selected() as usize;
            *active.borrow_mut() = Some((index, ps[index].x, ps[index].y));
            area.queue_draw();
        }
    ));
    drag.connect_drag_update(glib::clone!(
        #[strong]
        points,
        #[strong]
        active,
        #[weak]
        area,
        move |_, dx, dy| {
            if let Some((index, x, y)) = *active.borrow()
                && let Some(p) = points.borrow_mut().get_mut(index)
            {
                p.x = (x + dx / area.width() as f64).clamp(0., 1.);
                p.y = (y + dy / area.height() as f64).clamp(0., 1.);
            }
            area.queue_draw();
        }
    ));
    area.add_controller(drag);
    let remove = gtk::GestureClick::new();
    remove.set_button(3);
    remove.connect_pressed(glib::clone!(
        #[strong]
        points,
        #[weak]
        area,
        #[weak]
        side,
        move |_, _, x, y| {
            let index = hit(
                &points.borrow(),
                x / area.width() as f64,
                y / area.height() as f64,
                side.selected() == 1,
            );
            if let Some(i) = index {
                points.borrow_mut().remove(i);
            }
            area.queue_draw();
        }
    ));
    area.add_controller(remove);
    root.append(&area);
    let clear = Button::with_label("Limpiar mapa para un nuevo registro");
    clear.connect_clicked(glib::clone!(
        #[strong]
        points,
        #[weak]
        area,
        move |_| {
            points.borrow_mut().clear();
            area.queue_draw();
        }
    ));
    root.append(&clear);
    let status = Label::builder().wrap(true).build();
    if let Some(error) = &load_error {
        status.set_text(&format!("No se pudo leer el historial corporal: {error}"));
    }
    let dates = gtk::StringList::new(&[]);
    for s in history.borrow().iter() {
        dates.append(&s.date);
    }
    let selector = DropDown::builder().model(&dates).build();
    if !history.borrow().is_empty() {
        selector.set_selected(history.borrow().len() as u32 - 1);
    }
    selector.connect_selected_notify(glib::clone!(
        #[strong]
        history,
        #[strong]
        points,
        #[weak]
        area,
        move |dd| {
            if let Some(s) = history.borrow().get(dd.selected() as usize) {
                *points.borrow_mut() = s.points.clone();
                area.queue_draw();
            }
        }
    ));
    let save = Button::builder()
        .label("Guardar registro corporal")
        .css_classes(["suggested-action"])
        .sensitive(load_error.is_none())
        .build();
    save.connect_clicked(glib::clone!(
        #[strong]
        history,
        #[strong]
        points,
        #[strong]
        dates,
        #[weak]
        selector,
        #[weak]
        status,
        move |_| {
            let snapshot = Snapshot {
                date: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                points: points.borrow().clone(),
            };
            let mut next = history.borrow().clone();
            next.push(snapshot.clone());
            match persist(&next) {
                Ok(()) => {
                    *history.borrow_mut() = next;
                    dates.append(&snapshot.date);
                    selector.set_selected(history.borrow().len() as u32 - 1);
                    status.set_text("Registro guardado. Elige una fecha para comparar tus mapas.");
                }
                Err(e) => status.set_text(&format!("No se pudo guardar: {e}")),
            }
        }
    ));
    root.append(&save);
    root.append(&status);
    root.append(&Label::new(Some(
        "Historial corporal · selecciona un registro para verlo",
    )));
    root.append(&selector);
    gtk::ScrolledWindow::builder()
        .child(&root)
        .vexpand(true)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn points_keep_side_and_sensation() {
        let p = Point {
            x: 0.5,
            y: 0.3,
            back: true,
            kind: 5,
        };
        let json = serde_json::to_string(&p).unwrap();
        let copy: Point = serde_json::from_str(&json).unwrap();
        assert_eq!(hit(&[copy.clone()], 0.5, 0.3, false), None);
        assert_eq!(hit(&[copy], 0.5, 0.3, true), Some(0));
        assert_eq!(serde_json::from_str::<Point>(&json).unwrap().kind, 5);
    }
}
