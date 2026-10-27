use crate::body_regions::Model;
use adw::prelude::*;
use gtk::{Box, Button, DrawingArea, DropDown, Label, Orientation, glib};
use serde::{Deserialize, Serialize};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub(crate) const KINDS: [&str; 6] = [
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) back: bool,
    pub(crate) kind: usize,
    #[serde(default)]
    pub(crate) region_id: Option<u32>,
    #[serde(default)]
    pub(crate) intensity: Option<u8>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub(crate) date: String,
    pub(crate) points: Vec<Point>,
    /// Missing on old records, whose coordinates belong to the original silhouette.
    #[serde(default)]
    pub(crate) map_version: u8,
    #[serde(default)]
    pub(crate) moment: String,
    #[serde(default)]
    pub(crate) session_id: Option<String>,
}
fn hit(points: &[Point], x: f64, y: f64, back: bool, w: f64, h: f64) -> Option<usize> {
    points
        .iter()
        .rposition(|p| p.back == back && ((p.x - x) * w).hypot((p.y - y) * h) < 12.)
}
fn describe(label: &Label, model: &Model, index: Option<usize>) {
    if let Some(shape) = index.and_then(|i| model.shapes.get(i)) {
        label.set_text(&crate::i18n::tr(&format!(
            "{}\n{}",
            crate::i18n::tr(&shape.region.name),
            crate::i18n::tr(&shape.region.description)
        )));
    } else {
        label.set_text(&crate::i18n::tr("Señala una zona del cuerpo o elígela por su nombre. Izquierda y derecha corresponden a tu cuerpo."));
    }
}
fn summarize(label: &Label, points: &[Point], model: &Model, version: u8) {
    if points.is_empty() {
        label.set_text(&crate::i18n::tr("Sin puntos en este registro."));
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
                "● {} · {} · {} · {}",
                crate::i18n::tr(name),
                crate::i18n::tr(if p.back { "espalda" } else { "frente" }),
                crate::i18n::tr(KINDS[p.kind.min(KINDS.len() - 1)]),
                p.intensity
                    .map(|v| format!("{}/10", v.min(10)))
                    .unwrap_or_else(|| crate::i18n::tr("intensidad sin registrar"))
            )
        })
        .collect();
    label.set_text(&lines.join("\n"));
}

pub struct Panel {
    pub widget: gtk::ScrolledWindow,
    pub flush: Rc<dyn Fn() -> Result<(), String>>,
}
struct Linked {
    initial: Option<Snapshot>,
    moment: String,
    session_id: String,
    save: Rc<dyn Fn(Snapshot) -> Result<(), String>>,
}
pub fn build(repo: crate::storage::Repository, on_changed: Rc<dyn Fn()>) -> Panel {
    build_editor(repo, on_changed, None)
}
pub fn build_linked(
    repo: crate::storage::Repository,
    initial: Option<Snapshot>,
    moment: &str,
    session_id: String,
    save: Rc<dyn Fn(Snapshot) -> Result<(), String>>,
) -> Panel {
    build_editor(
        repo,
        Rc::new(|| {}),
        Some(Linked {
            initial,
            moment: moment.into(),
            session_id,
            save,
        }),
    )
}
fn build_editor(
    repo: crate::storage::Repository,
    on_changed: Rc<dyn Fn()>,
    linked: Option<Linked>,
) -> Panel {
    let linked = linked.map(Rc::new);
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
                    .label(crate::i18n::tr(&format!(
                        "No se pudo cargar el mapa corporal: {e}"
                    )))
                    .wrap(true)
                    .build(),
            );
            return Panel {
                widget: gtk::ScrolledWindow::builder().child(&root).build(),
                flush: Rc::new(|| Err("No se pudo cargar el mapa corporal".into())),
            };
        }
    };
    root.append(
        &Label::builder()
            .label(crate::i18n::tr("Mi cuerpo"))
            .css_classes(["title-1"])
            .build(),
    );
    root.append(&Label::builder().label(crate::i18n::tr("Elige una sensación y toca una zona para crear un punto. Arrastra para moverlo; selecciona un punto para editar su sensación e intensidad.")).wrap(true).build());
    let side = crate::i18n::dropdown(&["Frente", "Espalda"]);
    let kind = crate::i18n::dropdown(&KINDS);
    kind.set_tooltip_text(Some(&crate::i18n::tr(
        "Sensación para los puntos nuevos o seleccionados",
    )));
    let views = Box::builder()
        .orientation(Orientation::Horizontal)
        .halign(gtk::Align::Center)
        .css_classes(["linked"])
        .build();
    let front = gtk::ToggleButton::with_label(&crate::i18n::tr("Frente"));
    let back = gtk::ToggleButton::with_label(&crate::i18n::tr("Espalda"));
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
    let intensity = gtk::Scale::with_range(Orientation::Horizontal, 0., 10., 1.);
    intensity.set_value(5.);
    intensity.set_draw_value(true);
    intensity.set_digits(0);
    intensity.update_property(&[gtk::accessible::Property::Label(
        "Intensidad de la sensación de 0 a 10",
    )]);
    root.append(&Label::new(Some(&crate::i18n::tr("Intensidad · 0 a 10"))));
    root.append(&intensity);
    let moment = crate::i18n::dropdown(&[
        "Registro general",
        "Antes de la sesión",
        "Después de la sesión",
    ]);
    root.append(&moment);
    let heatmap = gtk::CheckButton::with_label(&crate::i18n::tr(
        "Ver calor por intensidad de la sensación elegida",
    ));
    root.append(&heatmap);
    let heat_filter = crate::i18n::dropdown(&KINDS);
    heat_filter.set_tooltip_text(Some(
        "Sensación que se muestra en el heatmap; no modifica tus puntos",
    ));
    heat_filter.set_visible(false);
    root.append(&heat_filter);
    root.append(&Label::builder().use_markup(true).wrap(true).label(crate::i18n::tr("<span foreground='#e63333'>● Dolor</span>   <span foreground='#f2a619'>● Tensión</span>   <span foreground='#8059d9'>● Cansancio</span>\n<span foreground='#e64d99'>● Irritado</span>   <span foreground='#2699e6'>● Trabajado</span>   <span foreground='#33b366'>● Fuerte</span>")).build());
    let history = Rc::new(RefCell::new(repo.all_maps()));
    let latest = if let Some(link) = &linked {
        link.initial.clone()
    } else {
        repo.snapshot().body_draft.or_else(|| {
            history
                .borrow()
                .last()
                .filter(|s| s.map_version == 1)
                .cloned()
        })
    };
    let points = Rc::new(RefCell::new(
        latest
            .as_ref()
            .map(|s| s.points.clone())
            .unwrap_or_default(),
    ));
    let version = Rc::new(Cell::new(latest.as_ref().map_or(1, |s| s.map_version)));
    let selected = Rc::new(Cell::new(None::<usize>));
    let selected_point = Rc::new(Cell::new(None::<usize>));
    let updating_controls = Rc::new(Cell::new(false));
    if let Some(snapshot) = &latest {
        moment.set_selected(match snapshot.moment.as_str() {
            "before" => 1,
            "after" => 2,
            _ => 0,
        });
    }
    if let Some(link) = &linked {
        moment.set_selected(if link.moment == "before" { 1 } else { 2 });
        moment.set_visible(false);
    }
    let hover = Rc::new(Cell::new(None::<usize>));
    let zone = Label::builder()
        .wrap(true)
        .xalign(0.)
        .css_classes(["stat-card"])
        .build();
    describe(&zone, &model, None);
    root.append(&zone);
    let mut names = vec![crate::i18n::tr("Seleccionar zona por nombre…")];
    names.extend(model.shapes.iter().map(|s| {
        format!(
            "{} · {}",
            crate::i18n::tr(if s.region.back { "Espalda" } else { "Frente" }),
            crate::i18n::tr(&s.region.name)
        )
    }));
    let regions = DropDown::from_strings(&names.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    regions.set_enable_search(true);
    regions.update_property(&[gtk::accessible::Property::Label(&crate::i18n::tr(
        "Zona del cuerpo",
    ))]);
    root.append(&regions);
    let area = DrawingArea::builder()
        .content_width(270)
        .content_height(600)
        .halign(gtk::Align::Center)
        .tooltip_text(crate::i18n::tr(
            "Selecciona una zona; clic derecho sobre un punto para eliminarlo",
        ))
        .build();
    area.update_property(&[gtk::accessible::Property::Label(
        "Mapa corporal con 73 zonas. También puedes elegir una zona por nombre.",
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
        #[weak]
        heat_filter,
        #[weak]
        heatmap,
        move |widget, cr, w, h| {
            cr.scale(w as f64, h as f64);
            if version.get() == 0 {
                crate::body_legacy::draw(widget, cr, side.selected() == 1);
            } else if heatmap.is_active() {
                let snapshot = Snapshot {
                    date: String::new(),
                    points: points.borrow().clone(),
                    map_version: version.get(),
                    moment: String::new(),
                    session_id: None,
                };
                model.draw_heat(
                    cr,
                    &widget.color(),
                    side.selected() == 1,
                    &crate::body_heat::values(&snapshot, heat_filter.selected() as usize),
                    heat_filter.selected() as usize,
                );
            } else {
                model.draw(
                    cr,
                    &widget.color(),
                    side.selected() == 1,
                    hover.get().or(selected.get()),
                );
            }
            for p in points.borrow().iter().filter(|p| {
                p.back == (side.selected() == 1)
                    && (!heatmap.is_active() || p.kind == heat_filter.selected() as usize)
            }) {
                let c = if heatmap.is_active() {
                    p.intensity
                        .map(|n| crate::body_heat::color(p.kind, n))
                        .unwrap_or((0.55, 0.55, 0.6))
                } else {
                    COLORS[p.kind.min(COLORS.len() - 1)]
                };
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
    heatmap.connect_toggled(glib::clone!(
        #[weak]
        area,
        #[weak]
        heat_filter,
        move |button| {
            heat_filter.set_visible(button.is_active());
            area.queue_draw();
        }
    ));
    heat_filter.connect_selected_notify(glib::clone!(
        #[weak]
        area,
        move |_| area.queue_draw()
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
        selected_point,
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
            selected_point.set(None);
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
    intensity.connect_value_changed(glib::clone!(
        #[strong]
        points,
        #[strong]
        selected_point,
        #[strong]
        updating_controls,
        #[strong]
        model,
        #[strong]
        version,
        #[weak]
        summary,
        #[weak]
        area,
        move |scale| {
            if updating_controls.get() {
                return;
            }
            {
                let mut ps = points.borrow_mut();
                if let Some(p) = selected_point.get().and_then(|i| ps.get_mut(i)) {
                    p.intensity = Some(scale.value() as u8);
                }
            }
            summarize(&summary, &points.borrow(), &model, version.get());
            area.queue_draw();
        }
    ));
    kind.connect_selected_notify(glib::clone!(
        #[strong]
        points,
        #[strong]
        selected_point,
        #[strong]
        updating_controls,
        #[strong]
        model,
        #[strong]
        version,
        #[weak]
        summary,
        #[weak]
        area,
        move |dd| {
            if updating_controls.get() {
                return;
            }
            {
                let mut ps = points.borrow_mut();
                if let Some(p) = selected_point.get().and_then(|i| ps.get_mut(i)) {
                    p.kind = dd.selected() as usize;
                }
            }
            summarize(&summary, &points.borrow(), &model, version.get());
            area.queue_draw();
        }
    ));
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
        #[weak]
        intensity,
        #[strong]
        updating_controls,
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
                        selected_point.set(None);
                        return;
                    }
                    ps.push(Point {
                        x,
                        y,
                        back,
                        kind: kind.selected() as usize,
                        intensity: Some(intensity.value() as u8),
                        region_id: region.map(|i| model.shapes[i].region.id),
                    });
                    ps.len() - 1
                }
            };
            let point_kind = ps[index].kind;
            let point_intensity = ps[index].intensity;
            let region = ps[index]
                .region_id
                .and_then(|id| model.shapes.iter().position(|s| s.region.id == id));
            selected.set(region);
            selected_point.set(Some(index));
            describe(&zone, &model, region);
            active.set(Some((index, ps[index].x, ps[index].y)));
            summarize(&summary, &ps, &model, version.get());
            drop(ps);
            updating_controls.set(true);
            kind.set_selected(point_kind.min(KINDS.len() - 1) as u32);
            intensity.set_value(f64::from(point_intensity.unwrap_or(5)));
            updating_controls.set(false);
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

    let mark = Button::with_label(&crate::i18n::tr("Marcar zona seleccionada"));
    mark.connect_clicked(glib::clone!(#[strong] points,#[strong] model,#[strong] version,#[strong] selected,#[strong] selected_point,#[weak] kind,#[weak] intensity,#[weak] area,#[weak] summary,#[weak] status,move|_|{
        if version.get()==0{status.set_text(&crate::i18n::tr("Este registro usa el mapa anterior. Inicia un nuevo registro para usar las zonas detalladas."));return;}
        if let Some(i)=selected.get(){let shape=&model.shapes[i];let mut ps=points.borrow_mut();ps.push(Point{x:shape.center.0,y:shape.center.1,back:shape.region.back,kind:kind.selected() as usize,intensity:Some(intensity.value() as u8),region_id:Some(shape.region.id)});selected_point.set(Some(ps.len()-1));summarize(&summary,&ps,&model,version.get());area.queue_draw();}else{status.set_text(&crate::i18n::tr("Primero selecciona una zona por su nombre o en el dibujo."));}
    }));
    root.append(&mark);
    let apply = Button::with_label(&crate::i18n::tr(
        "Aplicar sensación e intensidad al punto seleccionado",
    ));
    apply.connect_clicked(glib::clone!(
        #[strong]
        points,
        #[strong]
        selected_point,
        #[strong]
        model,
        #[strong]
        version,
        #[weak]
        kind,
        #[weak]
        intensity,
        #[weak]
        summary,
        #[weak]
        area,
        move |_| {
            {
                let mut ps = points.borrow_mut();
                if let Some(p) = selected_point.get().and_then(|i| ps.get_mut(i)) {
                    p.kind = kind.selected() as usize;
                    p.intensity = Some(intensity.value() as u8);
                }
            }
            summarize(&summary, &points.borrow(), &model, version.get());
            area.queue_draw();
        }
    ));
    root.append(&apply);
    let delete = Button::with_label(&crate::i18n::tr("Eliminar punto seleccionado"));
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
    let clear = Button::with_label(&crate::i18n::tr("Nuevo registro · limpiar mapa"));
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
            status.set_text(&crate::i18n::tr(
                "Nuevo registro corporal. El historial se conserva.",
            ));
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
    selector.connect_selected_notify(glib::clone!(#[strong] history,#[strong] points,#[strong] version,#[strong] selected,#[strong] selected_point,#[strong] model,#[weak] area,#[weak] summary,#[weak] zone,#[weak] status,#[weak] moment,move|dd|{
        if let Some(s)=history.borrow().get(dd.selected() as usize){
            *points.borrow_mut()=s.points.clone();version.set(s.map_version);moment.set_selected(match s.moment.as_str(){"before"=>1,"after"=>2,_=>0});selected.set(None);selected_point.set(None);area.set_content_height(if s.map_version==0{430}else{600});describe(&zone,&model,None);summarize(&summary,&points.borrow(),&model,version.get());
            status.set_text(&crate::i18n::tr(if s.map_version==0{"Registro anterior: se muestra la silueta original para conservar la ubicación de tus puntos."}else{"Registro corporal cargado. Guardar crea una entrada nueva."}));area.queue_draw();
        }
    }));
    let evolution = crate::body_evolution::build(history.clone(), model.clone());
    let refresh_evolution = evolution.refresh.clone();
    let baseline = Rc::new(RefCell::new((
        points.borrow().clone(),
        version.get(),
        moment.selected(),
    )));
    let flush: Rc<dyn Fn() -> Result<(), String>> = Rc::new(glib::clone!(
        #[strong]
        points,
        #[strong]
        version,
        #[strong]
        baseline,
        #[strong]
        repo,
        #[strong]
        linked,
        #[weak]
        moment,
        #[weak]
        status,
        #[upgrade_or]
        Err("El editor del mapa ya está cerrado".into()),
        move || {
            let signature = (points.borrow().clone(), version.get(), moment.selected());
            if *baseline.borrow() == signature {
                return Ok(());
            }
            let snapshot = Snapshot {
                date: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                points: signature.0.clone(),
                map_version: signature.1,
                moment: match signature.2 {
                    1 => "before",
                    2 => "after",
                    _ => "general",
                }
                .into(),
                session_id: linked.as_ref().map(|l| l.session_id.clone()),
            };
            let result = if let Some(link) = &linked {
                (link.save)(snapshot)
            } else {
                repo.update(|data| {
                    data.body_draft = Some(snapshot);
                    Ok(())
                })
            };
            match &result {
                Ok(()) => {
                    *baseline.borrow_mut() = signature;
                    status.set_text(&crate::i18n::tr(
                        "Borrador del mapa guardado en este equipo.",
                    ));
                }
                Err(e) => status.set_text(&crate::i18n::tr(&format!(
                    "No se pudo guardar el borrador. Tus puntos siguen aquí: {e}"
                ))),
            }
            result
        }
    ));
    let weak_area = area.downgrade();
    glib::timeout_add_local(
        std::time::Duration::from_millis(600),
        glib::clone!(
            #[strong]
            flush,
            move || {
                if weak_area.upgrade().is_none() {
                    return glib::ControlFlow::Break;
                }
                let _ = flush();
                glib::ControlFlow::Continue
            }
        ),
    );
    let save = Button::builder()
        .label(crate::i18n::tr(if linked.is_some() {
            "Guardar mapa en el borrador"
        } else {
            "Guardar registro corporal"
        }))
        .css_classes(["suggested-action"])
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
        #[strong]
        refresh_evolution,
        #[strong]
        baseline,
        #[strong]
        repo,
        #[strong]
        linked,
        #[strong]
        on_changed,
        #[weak]
        moment,
        #[weak]
        selector,
        #[weak]
        status,
        move |_| {
            let snapshot = Snapshot {
                date: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                points: points.borrow().clone(),
                map_version: version.get(),
                session_id: linked.as_ref().map(|l| l.session_id.clone()),
                moment: match moment.selected() {
                    1 => "before",
                    2 => "after",
                    _ => "general",
                }
                .into(),
            };
            let result = if let Some(link) = &linked {
                (link.save)(snapshot.clone())
            } else {
                repo.update(|data| {
                    data.body.push(snapshot.clone());
                    data.body_draft = None;
                    Ok(())
                })
            };
            match result {
                Ok(()) => {
                    *baseline.borrow_mut() = (
                        snapshot.points.clone(),
                        snapshot.map_version,
                        moment.selected(),
                    );
                    if linked.is_none() {
                        history.borrow_mut().push(snapshot.clone());
                        dates.append(&snapshot.date);
                        let index = history.borrow().len() as u32 - 1;
                        selector.set_selected(index);
                        refresh_evolution();
                    }
                    status.set_text(&crate::i18n::tr(if linked.is_some() {
                        "Mapa guardado en el borrador de la sesión."
                    } else {
                        "Registro corporal guardado."
                    }));
                    on_changed();
                }
                Err(e) => status.set_text(&crate::i18n::tr(&format!(
                    "No se pudo guardar. Tus puntos siguen aquí: {e}"
                ))),
            }
        }
    ));
    root.append(&save);
    root.append(&status);
    let widget = if linked.is_some() {
        gtk::ScrolledWindow::builder()
            .child(&root)
            .vexpand(true)
            .build()
    } else {
        root.append(&Label::new(Some(
            "Historial corporal · selecciona un registro para verlo",
        )));
        root.append(&selector);
        let stack = gtk::Stack::new();
        stack.set_vhomogeneous(false);
        stack.add_titled(&root, Some("registrar"), &crate::i18n::tr("Registrar"));
        stack.add_titled(
            &evolution.widget,
            Some("evolucion"),
            &crate::i18n::tr("Evolución"),
        );
        let switcher = gtk::StackSwitcher::builder()
            .stack(&stack)
            .halign(gtk::Align::Center)
            .build();
        let content = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .build();
        content.append(&switcher);
        content.append(&stack);
        gtk::ScrolledWindow::builder()
            .child(&content)
            .vexpand(true)
            .build()
    };
    Panel { widget, flush }
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
        assert_eq!(s.points[0].intensity, None);
        let p = &s.points[0];
        assert_eq!(hit(&s.points, p.x, p.y, false, 270., 430.), None);
        assert_eq!(hit(&s.points, p.x, p.y, true, 270., 430.), Some(0));
    }
    #[test]
    fn svg_records_keep_region_side_and_sensation() {
        let s = Snapshot {
            date: "2026-10-27".into(),
            map_version: 1,
            moment: "before".into(),
            session_id: None,
            points: vec![Point {
                x: 0.5,
                y: 0.3,
                back: true,
                kind: 5,
                region_id: Some(39),
                intensity: Some(6),
            }],
        };
        let copy: Snapshot = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(copy.map_version, 1);
        assert_eq!(copy.points[0].region_id, Some(39));
        assert_eq!(copy.points[0].kind, 5);
        assert_eq!(copy.points[0].intensity, Some(6));
        assert_eq!(copy.moment, "before");
        assert!(copy.points[0].back);
    }
}
