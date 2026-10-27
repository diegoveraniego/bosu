use crate::body_regions::Model;
use adw::prelude::*;
use gtk::{Box, Button, DrawingArea, DropDown, Label, Orientation, glib};
use serde::{Deserialize, Serialize};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

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
    #[serde(default)]
    region_id: Option<u32>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Snapshot {
    date: String,
    points: Vec<Point>,
    /// Missing on old records, whose coordinates belong to the original silhouette.
    #[serde(default)]
    map_version: u8,
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
fn hit(points: &[Point], x: f64, y: f64, back: bool, w: f64, h: f64) -> Option<usize> {
    points
        .iter()
        .rposition(|p| p.back == back && ((p.x - x) * w).hypot((p.y - y) * h) < 12.)
}
fn describe(label: &Label, model: &Model, index: Option<usize>) {
    if let Some(shape) = index.and_then(|i| model.shapes.get(i)) {
        label.set_text(&format!(
            "{}\n{}",
            shape.region.name, shape.region.description
        ));
    } else {
        label.set_text("Señala una zona del cuerpo o elígela por su nombre. Izquierda y derecha corresponden a tu cuerpo.");
    }
}
fn summarize(label: &Label, points: &[Point], model: &Model, version: u8) {
    if points.is_empty() {
        label.set_text("Sin puntos en este registro.");
        return;
    }
    let lines: Vec<_> = points
        .iter()
        .map(|p| {
            let name = if version == 0 {
                "Ubicación del mapa anterior"
            } else {
                model
                    .shapes
                    .iter()
                    .find(|s| Some(s.region.id) == p.region_id)
                    .map(|s| s.region.name.as_str())
                    .unwrap_or("Zona sin identificar")
            };
            format!(
                "● {} · {} · {}",
                name,
                if p.back { "espalda" } else { "frente" },
                KINDS[p.kind.min(KINDS.len() - 1)]
            )
        })
        .collect();
    label.set_text(&lines.join("\n"));
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
    let model = match Model::load() {
        Ok(m) => Rc::new(m),
        Err(e) => {
            root.append(
                &Label::builder()
                    .label(format!("No se pudo cargar el mapa SVG: {e}"))
                    .wrap(true)
                    .build(),
            );
            return gtk::ScrolledWindow::builder().child(&root).build();
        }
    };
    root.append(
        &Label::builder()
            .label("Mi cuerpo")
            .css_classes(["title-1"])
            .build(),
    );
    root.append(&Label::builder().label("Elige una sensación y toca una zona para crear un punto. Arrastra para moverlo; toca un punto para cambiar su sensación.").wrap(true).build());
    let side = DropDown::from_strings(&["Frente", "Espalda"]);
    let kind = DropDown::from_strings(&KINDS);
    kind.set_tooltip_text(Some("Sensación para los puntos nuevos o seleccionados"));
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
        move |b| if b.is_active() {
            side.set_selected(0);
        }
    ));
    back.connect_toggled(glib::clone!(
        #[strong]
        side,
        move |b| if b.is_active() {
            side.set_selected(1);
        }
    ));
    views.append(&front);
    views.append(&back);
    root.append(&views);
    root.append(&kind);
    root.append(&Label::builder().use_markup(true).wrap(true).label("<span foreground='#e63333'>● Dolor</span>   <span foreground='#f2a619'>● Tensión</span>   <span foreground='#8059d9'>● Cansancio</span>\n<span foreground='#e64d99'>● Irritado</span>   <span foreground='#2699e6'>● Trabajado</span>   <span foreground='#33b366'>● Fuerte</span>").build());
    let (initial, load_error) = match std::fs::read(path()) {
        Ok(bytes) => match serde_json::from_slice::<Vec<Snapshot>>(&bytes) {
            Ok(h) => (h, None),
            Err(e) => (vec![], Some(e.to_string())),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (vec![], None),
        Err(e) => (vec![], Some(e.to_string())),
    };
    let history = Rc::new(RefCell::new(initial));
    // Start on the detailed model. Old coordinates are displayed only on their original model.
    let latest = history
        .borrow()
        .last()
        .filter(|s| s.map_version == 1)
        .cloned();
    let points = Rc::new(RefCell::new(
        latest
            .as_ref()
            .map(|s| s.points.clone())
            .unwrap_or_default(),
    ));
    let version = Rc::new(Cell::new(1u8));
    let selected = Rc::new(Cell::new(None::<usize>));
    let selected_point = Rc::new(Cell::new(None::<usize>));
    let hover = Rc::new(Cell::new(None::<usize>));
    let zone = Label::builder()
        .wrap(true)
        .xalign(0.)
        .css_classes(["stat-card"])
        .build();
    describe(&zone, &model, None);
    root.append(&zone);
    let mut names = vec!["Seleccionar zona por nombre…".to_string()];
    names.extend(model.shapes.iter().map(|s| {
        format!(
            "{} · {}",
            if s.region.back { "Espalda" } else { "Frente" },
            s.region.name
        )
    }));
    let regions = DropDown::from_strings(&names.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    regions.set_enable_search(true);
    regions.update_property(&[gtk::accessible::Property::Label("Zona del cuerpo")]);
    root.append(&regions);
    let area = DrawingArea::builder()
        .content_width(270)
        .content_height(600)
        .halign(gtk::Align::Center)
        .tooltip_text("Selecciona una zona; clic derecho sobre un punto para eliminarlo")
        .build();
    area.update_property(&[gtk::accessible::Property::Label(
        "Mapa corporal SVG con 73 zonas. También puedes elegir una zona por nombre.",
    )]);
    area.set_draw_func(glib::clone!(
        #[strong]
        points,
        #[strong]
        model,
        #[strong]
        version,
        #[strong]
        selected,
        #[strong]
        hover,
        #[weak]
        side,
        move |widget, cr, w, h| {
            cr.scale(w as f64, h as f64);
            if version.get() == 0 {
                crate::body_legacy::draw(widget, cr, side.selected() == 1);
            } else {
                model.draw(
                    cr,
                    &widget.color(),
                    side.selected() == 1,
                    hover.get().or(selected.get()),
                );
            }
            for p in points
                .borrow()
                .iter()
                .filter(|p| p.back == (side.selected() == 1))
            {
                let c = COLORS[p.kind.min(COLORS.len() - 1)];
                cr.set_source_rgb(c.0, c.1, c.2);
                let _ = cr.save();
                cr.translate(p.x, p.y);
                cr.scale(7. / w as f64, 7. / h as f64);
                cr.arc(0., 0., 1., 0., std::f64::consts::TAU);
                let _ = cr.fill_preserve();
                cr.set_source_rgb(1., 1., 1.);
                cr.set_line_width(0.18);
                let _ = cr.stroke();
                let _ = cr.restore();
            }
        }
    ));
    side.connect_selected_notify(glib::clone!(
        #[weak]
        area,
        #[weak]
        front,
        #[weak]
        back,
        #[weak]
        zone,
        #[strong]
        selected,
        #[strong]
        hover,
        #[strong]
        model,
        move |dd| {
            if dd.selected() == 1 {
                back.set_active(true);
            } else {
                front.set_active(true);
            }
            hover.set(None);
            selected.set(None);
            describe(&zone, &model, None);
            area.queue_draw();
        }
    ));
    regions.connect_selected_notify(glib::clone!(
        #[strong]
        selected,
        #[strong]
        selected_point,
        #[strong]
        model,
        #[weak]
        side,
        #[weak]
        area,
        #[weak]
        zone,
        move |dd| {
            let index = dd
                .selected()
                .checked_sub(1)
                .map(|i| i as usize)
                .filter(|i| *i < model.shapes.len());
            if let Some(i) = index {
                side.set_selected(u32::from(model.shapes[i].region.back));
            }
            selected.set(index);
            selected_point.set(None);
            describe(&zone, &model, index);
            area.queue_draw();
        }
    ));
    let summary = Label::builder()
        .wrap(true)
        .xalign(0.)
        .selectable(true)
        .build();
    summarize(&summary, &points.borrow(), &model, version.get());
    let motion = gtk::EventControllerMotion::new();
    motion.connect_motion(glib::clone!(
        #[strong]
        model,
        #[strong]
        hover,
        #[strong]
        version,
        #[weak]
        area,
        #[weak]
        zone,
        #[weak]
        side,
        move |_, x, y| {
            let index = if version.get() == 1 {
                model.locate(
                    x / area.width() as f64,
                    y / area.height() as f64,
                    side.selected() == 1,
                )
            } else {
                None
            };
            if hover.replace(index) != index {
                describe(&zone, &model, index);
                area.queue_draw();
            }
        }
    ));
    motion.connect_leave(glib::clone!(
        #[strong]
        hover,
        #[strong]
        selected,
        #[strong]
        model,
        #[weak]
        zone,
        #[weak]
        area,
        move |_| {
            hover.set(None);
            describe(&zone, &model, selected.get());
            area.queue_draw();
        }
    ));
    area.add_controller(motion);
    let drag = gtk::GestureDrag::new();
    let active = Rc::new(Cell::new(None::<(usize, f64, f64)>));
    drag.connect_drag_begin(glib::clone!(
        #[strong]
        points,
        #[strong]
        active,
        #[strong]
        model,
        #[strong]
        version,
        #[strong]
        selected,
        #[strong]
        selected_point,
        #[weak]
        area,
        #[weak]
        side,
        #[weak]
        kind,
        #[weak]
        zone,
        #[weak]
        summary,
        move |_, x, y| {
            active.set(None);
            let x = x / area.width() as f64;
            let y = y / area.height() as f64;
            let back = side.selected() == 1;
            let region = if version.get() == 1 {
                model.locate(x, y, back)
            } else {
                None
            };
            let mut ps = points.borrow_mut();
            let existing = hit(&ps, x, y, back, area.width() as f64, area.height() as f64);
            let index = match existing {
                Some(i) => i,
                None => {
                    if version.get() == 1 && region.is_none() {
                        return;
                    }
                    ps.push(Point {
                        x,
                        y,
                        back,
                        kind: kind.selected() as usize,
                        region_id: region.map(|i| model.shapes[i].region.id),
                    });
                    ps.len() - 1
                }
            };
            ps[index].kind = kind.selected() as usize;
            let region = ps[index]
                .region_id
                .and_then(|id| model.shapes.iter().position(|s| s.region.id == id));
            selected.set(region);
            selected_point.set(Some(index));
            describe(&zone, &model, region);
            active.set(Some((index, ps[index].x, ps[index].y)));
            summarize(&summary, &ps, &model, version.get());
            area.queue_draw();
        }
    ));
    drag.connect_drag_update(glib::clone!(
        #[strong]
        points,
        #[strong]
        active,
        #[strong]
        model,
        #[strong]
        version,
        #[strong]
        selected,
        #[weak]
        area,
        #[weak]
        zone,
        #[weak]
        summary,
        move |_, dx, dy| {
            if let Some((i, x, y)) = active.get()
                && let Some(p) = points.borrow_mut().get_mut(i)
            {
                let x = (x + dx / area.width() as f64).clamp(0., 1.);
                let y = (y + dy / area.height() as f64).clamp(0., 1.);
                let region = model.locate(x, y, p.back);
                if version.get() == 1 && region.is_none() {
                    return;
                }
                p.x = x;
                p.y = y;
                if version.get() == 1 {
                    p.region_id = region.map(|j| model.shapes[j].region.id);
                }
                selected.set(region);
                describe(&zone, &model, region);
            }
            summarize(&summary, &points.borrow(), &model, version.get());
            area.queue_draw();
        }
    ));
    area.add_controller(drag);
    let remove = gtk::GestureClick::new();
    remove.set_button(3);
    remove.connect_pressed(glib::clone!(
        #[strong]
        points,
        #[strong]
        model,
        #[strong]
        version,
        #[strong]
        selected_point,
        #[weak]
        area,
        #[weak]
        side,
        #[weak]
        summary,
        move |_, _, x, y| {
            let index = hit(
                &points.borrow(),
                x / area.width() as f64,
                y / area.height() as f64,
                side.selected() == 1,
                area.width() as f64,
                area.height() as f64,
            );
            if let Some(i) = index {
                points.borrow_mut().remove(i);
            }
            selected_point.set(None);
            summarize(&summary, &points.borrow(), &model, version.get());
            area.queue_draw();
        }
    ));
    area.add_controller(remove);
    root.append(&area);
    let status = Label::builder().wrap(true).build();
    if let Some(e) = &load_error {
        status.set_text(&format!("No se pudo leer el historial corporal: {e}"));
    }
    let mark = Button::with_label("Marcar zona seleccionada");
    mark.connect_clicked(glib::clone!(#[strong] points,#[strong] model,#[strong] version,#[strong] selected,#[strong] selected_point,#[weak] kind,#[weak] area,#[weak] summary,#[weak] status,move|_|{
        if version.get()==0{status.set_text("Este registro usa el mapa anterior. Inicia un nuevo registro para usar las zonas SVG.");return;}
        if let Some(i)=selected.get(){let shape=&model.shapes[i];let mut ps=points.borrow_mut();ps.push(Point{x:shape.center.0,y:shape.center.1,back:shape.region.back,kind:kind.selected() as usize,region_id:Some(shape.region.id)});selected_point.set(Some(ps.len()-1));summarize(&summary,&ps,&model,version.get());area.queue_draw();}else{status.set_text("Primero selecciona una zona por su nombre o en el dibujo.");}
    }));
    root.append(&mark);
    let delete = Button::with_label("Eliminar punto seleccionado");
    delete.connect_clicked(glib::clone!(
        #[strong]
        points,
        #[strong]
        selected_point,
        #[strong]
        model,
        #[strong]
        version,
        #[weak]
        area,
        #[weak]
        summary,
        move |_| {
            {
                let mut ps = points.borrow_mut();
                if let Some(i) = selected_point.take()
                    && i < ps.len()
                {
                    ps.remove(i);
                }
            }
            summarize(&summary, &points.borrow(), &model, version.get());
            area.queue_draw();
        }
    ));
    root.append(&delete);
    root.append(&summary);
    let clear = Button::with_label("Nuevo registro · limpiar mapa");
    clear.connect_clicked(glib::clone!(
        #[strong]
        points,
        #[strong]
        version,
        #[strong]
        selected,
        #[strong]
        selected_point,
        #[strong]
        model,
        #[weak]
        area,
        #[weak]
        zone,
        #[weak]
        summary,
        #[weak]
        status,
        move |_| {
            points.borrow_mut().clear();
            version.set(1);
            selected.set(None);
            selected_point.set(None);
            area.set_content_height(600);
            describe(&zone, &model, None);
            summarize(&summary, &points.borrow(), &model, 1);
            status.set_text("Nuevo registro en el mapa SVG. El historial se conserva.");
            area.queue_draw();
        }
    ));
    root.append(&clear);
    let dates = gtk::StringList::new(&[]);
    for s in history.borrow().iter() {
        dates.append(&s.date);
    }
    let selector = DropDown::builder().model(&dates).build();
    selector.set_selected(gtk::INVALID_LIST_POSITION);
    if latest.is_some() {
        selector.set_selected(history.borrow().len() as u32 - 1);
    }
    clear.connect_clicked(glib::clone!(
        #[weak]
        selector,
        #[weak]
        regions,
        move |_| {
            selector.set_selected(gtk::INVALID_LIST_POSITION);
            regions.set_selected(0);
        }
    ));
    selector.connect_selected_notify(glib::clone!(#[strong] history,#[strong] points,#[strong] version,#[strong] selected,#[strong] selected_point,#[strong] model,#[weak] area,#[weak] summary,#[weak] zone,#[weak] status,move|dd|{
        if let Some(s)=history.borrow().get(dd.selected() as usize){
            *points.borrow_mut()=s.points.clone();version.set(s.map_version);selected.set(None);selected_point.set(None);area.set_content_height(if s.map_version==0{430}else{600});describe(&zone,&model,None);summarize(&summary,&points.borrow(),&model,version.get());
            status.set_text(if s.map_version==0{"Registro anterior: se muestra la silueta original para conservar la ubicación de tus puntos."}else{"Registro SVG cargado. Guardar crea una entrada nueva."});area.queue_draw();
        }
    }));
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
        version,
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
                map_version: version.get(),
            };
            let mut next = history.borrow().clone();
            next.push(snapshot.clone());
            match persist(&next) {
                Ok(()) => {
                    *history.borrow_mut() = next;
                    dates.append(&snapshot.date);
                    let index = history.borrow().len() as u32 - 1;
                    selector.set_selected(index);
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
    fn old_history_keeps_original_coordinate_system() {
        let s: Snapshot = serde_json::from_str(
            r#"{"date":"2026-10-13","points":[{"x":0.5,"y":0.3,"back":true,"kind":5}]}"#,
        )
        .unwrap();
        assert_eq!(s.map_version, 0);
        assert_eq!(s.points[0].region_id, None);
        let p = &s.points[0];
        assert_eq!(hit(&s.points, p.x, p.y, false, 270., 430.), None);
        assert_eq!(hit(&s.points, p.x, p.y, true, 270., 430.), Some(0));
    }
    #[test]
    fn svg_records_keep_region_side_and_sensation() {
        let s = Snapshot {
            date: "2026-10-27".into(),
            map_version: 1,
            points: vec![Point {
                x: 0.5,
                y: 0.3,
                back: true,
                kind: 5,
                region_id: Some(39),
            }],
        };
        let copy: Snapshot = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(copy.map_version, 1);
        assert_eq!(copy.points[0].region_id, Some(39));
        assert_eq!(copy.points[0].kind, 5);
        assert!(copy.points[0].back);
    }
}
