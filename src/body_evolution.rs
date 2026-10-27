use crate::{
    body_gif, body_heat,
    body_map::{KINDS, Snapshot},
    body_regions::Model,
};
use adw::prelude::*;
use gtk::{Box, Button, DrawingArea, DropDown, Label, Orientation, Scale, glib};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub struct Panel {
    pub widget: Box,
    pub refresh: Rc<dyn Fn()>,
}
fn frames(history: &[Snapshot], days: Option<i64>, moment: u32) -> Vec<Snapshot> {
    let today = chrono::Local::now().date_naive();
    let cutoff = days.map(|n| today - chrono::Duration::days(n - 1));
    let mut result: Vec<_> = history
        .iter()
        .filter(|s| s.map_version == 1)
        .filter(|s| match moment {
            1 => s.moment.is_empty() || s.moment == "general",
            2 => s.moment == "before",
            3 => s.moment == "after",
            _ => true,
        })
        .filter(|s| {
            cutoff.is_none_or(|date| {
                s.date
                    .get(..10)
                    .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                    .is_some_and(|d| d >= date && d <= today)
            })
        })
        .cloned()
        .collect();
    result.sort_by(|a, b| a.date.cmp(&b.date));
    result
}
fn maximum(snapshot: &Snapshot, kind: usize, region: Option<u32>) -> Option<u8> {
    body_heat::values(snapshot, kind)
        .into_iter()
        .filter(|(id, _)| region.is_none_or(|r| r == *id))
        .filter_map(|(_, v)| v)
        .max()
}
pub fn build(history: Rc<RefCell<Vec<Snapshot>>>, model: Rc<Model>) -> Panel {
    let root = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(24)
        .build();
    root.append(
        &Label::builder()
            .label("Evolución corporal")
            .css_classes(["title-2"])
            .build(),
    );
    root.append(&Label::builder().label("Recorre tus registros o reprodúcelos como una animación. La escala 0–10 es la misma en todas las fechas. Zona sin marcar = sin datos, no ausencia de dolor; ? = intensidad sin registrar.").wrap(true).build());
    let kind = DropDown::from_strings(&KINDS);
    root.append(&kind);
    let range = DropDown::from_strings(&[
        "Todo el historial",
        "Últimos 7 días",
        "Últimos 30 días",
        "Últimos 90 días",
    ]);
    root.append(&range);
    let moment = DropDown::from_strings(&[
        "Todos los momentos",
        "Registro general",
        "Antes de la sesión",
        "Después de la sesión",
    ]);
    root.append(&moment);
    let visible = Rc::new(RefCell::new(Vec::<Snapshot>::new()));
    let index = Rc::new(Cell::new(0usize));
    let area = DrawingArea::builder()
        .content_width(350)
        .content_height(510)
        .halign(gtk::Align::Center)
        .build();
    area.update_property(&[gtk::accessible::Property::Label(
        "Heatmap de frente y espalda del registro seleccionado",
    )]);
    area.set_draw_func(glib::clone!(
        #[strong]
        visible,
        #[strong]
        index,
        #[strong]
        model,
        #[weak]
        kind,
        move |widget, cr, w, h| {
            if let Some(snapshot) = visible.borrow().get(index.get()) {
                body_gif::render(
                    cr,
                    &model,
                    snapshot,
                    kind.selected() as usize,
                    w as f64,
                    h as f64,
                    &widget.color(),
                );
            }
        }
    ));
    root.append(&area);
    let detail = Label::builder().wrap(true).xalign(0.).build();
    root.append(&detail);
    let timeline = Scale::with_range(Orientation::Horizontal, 0., 1., 1.);
    timeline.set_draw_value(false);
    timeline.set_digits(0);
    timeline.update_property(&[gtk::accessible::Property::Label("Registro del historial")]);
    root.append(&timeline);
    let controls = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .halign(gtk::Align::Center)
        .build();
    let previous = Button::with_label("Anterior");
    let play = gtk::ToggleButton::with_label("Reproducir");
    let next = Button::with_label("Siguiente");
    controls.append(&previous);
    controls.append(&play);
    controls.append(&next);
    root.append(&controls);
    let mut names = vec!["Máximo de todas las zonas".into()];
    names.extend(model.shapes.iter().map(|s| {
        format!(
            "{} · {}",
            s.region.name,
            if s.region.back { "espalda" } else { "frente" }
        )
    }));
    let region = DropDown::from_strings(&names.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    region.set_enable_search(true);
    root.append(&Label::new(Some(
        "Gráfico de intensidad máxima por registro",
    )));
    root.append(&region);
    let chart = DrawingArea::builder()
        .content_width(350)
        .content_height(150)
        .hexpand(true)
        .build();
    chart.set_draw_func(glib::clone!(
        #[strong]
        visible,
        #[strong]
        model,
        #[strong]
        index,
        #[weak]
        kind,
        #[weak]
        region,
        move |widget, cr, w, h| {
            let ink = widget.color();
            let data = visible.borrow();
            let x0 = 30.;
            let y0 = 15.;
            let width = (w as f64 - 45.).max(1.);
            let height = (h as f64 - 35.).max(1.);
            cr.set_source_rgba(ink.red() as f64, ink.green() as f64, ink.blue() as f64, 0.4);
            cr.set_font_size(10.);
            for value in [0, 5, 10] {
                let y = y0 + height * (1. - f64::from(value) / 10.);
                cr.move_to(x0, y);
                cr.line_to(x0 + width, y);
                let _ = cr.stroke();
                cr.move_to(8., y + 3.);
                let _ = cr.show_text(&value.to_string());
            }
            let selected = region
                .selected()
                .checked_sub(1)
                .and_then(|i| model.shapes.get(i as usize))
                .map(|s| s.region.id);
            let mut last = None;
            for (i, s) in data.iter().enumerate() {
                let x = x0 + width * i as f64 / (data.len().saturating_sub(1).max(1)) as f64;
                if let Some(value) = maximum(s, kind.selected() as usize, selected) {
                    let y = y0 + height * (1. - f64::from(value) / 10.);
                    let c = body_heat::color(kind.selected() as usize, value);
                    cr.set_source_rgb(c.0, c.1, c.2);
                    cr.set_line_width(2.);
                    if let Some((px, py)) = last {
                        cr.move_to(px, py);
                        cr.line_to(x, y);
                        let _ = cr.stroke();
                    }
                    cr.arc(
                        x,
                        y,
                        if i == index.get() { 5. } else { 3. },
                        0.,
                        std::f64::consts::TAU,
                    );
                    let _ = cr.fill();
                    last = Some((x, y));
                } else {
                    last = None;
                }
            }
        }
    ));
    root.append(&chart);
    root.append(&Label::builder().label("Un punto por registro, de más antiguo a más reciente. Los huecos indican intensidad sin registrar. Si hay varios puntos de la misma sensación en una zona se usa el máximo.").wrap(true).build());
    let update: Rc<dyn Fn()> = Rc::new(glib::clone!(
        #[strong]
        visible,
        #[strong]
        index,
        #[weak]
        kind,
        #[weak]
        area,
        #[weak]
        chart,
        #[weak]
        detail,
        move || {
            if let Some(s) = visible.borrow().get(index.get()) {
                let count = visible.borrow().len();
                let max = maximum(s, kind.selected() as usize, None)
                    .map(|n| format!("{n}/10"))
                    .unwrap_or("sin intensidad registrada".into());
                detail.set_text(&format!(
                    "Registro {}/{} · {} · {}\n{}: máximo {}",
                    index.get() + 1,
                    count,
                    s.date,
                    body_gif::moment_label(&s.moment),
                    KINDS[kind.selected() as usize],
                    max
                ));
            } else {
                detail.set_text("No hay registros SVG en este período. Guarda un mapa con intensidad para comenzar.");
            }
            area.queue_draw();
            chart.queue_draw();
        }
    ));
    timeline.connect_value_changed(glib::clone!(
        #[strong]
        index,
        #[strong]
        visible,
        #[strong]
        update,
        move |scale| {
            index.set((scale.value() as usize).min(visible.borrow().len().saturating_sub(1)));
            update();
        }
    ));
    previous.connect_clicked(glib::clone!(
        #[weak]
        timeline,
        move |_| timeline.set_value((timeline.value() - 1.).max(0.))
    ));
    next.connect_clicked(glib::clone!(
        #[weak]
        timeline,
        #[strong]
        visible,
        move |_| timeline.set_value(
            (timeline.value() + 1.).min(visible.borrow().len().saturating_sub(1) as f64)
        )
    ));
    play.connect_toggled(|b| {
        b.set_label(if b.is_active() {
            "Pausar"
        } else {
            "Reproducir"
        })
    });
    let weak_timeline = timeline.downgrade();
    let weak_play = play.downgrade();
    glib::timeout_add_local(
        std::time::Duration::from_millis(800),
        glib::clone!(
            #[strong]
            visible,
            move || {
                let (Some(timeline), Some(play)) = (weak_timeline.upgrade(), weak_play.upgrade())
                else {
                    return glib::ControlFlow::Break;
                };
                if play.is_active() {
                    let len = visible.borrow().len();
                    if len > 1 {
                        timeline.set_value(((timeline.value() as usize + 1) % len) as f64);
                    } else {
                        play.set_active(false);
                    }
                }
                glib::ControlFlow::Continue
            }
        ),
    );
    region.connect_selected_notify(glib::clone!(
        #[strong]
        update,
        move |_| update()
    ));
    kind.connect_selected_notify(glib::clone!(
        #[strong]
        update,
        move |_| update()
    ));
    let export_busy = Rc::new(Cell::new(false));
    let export = Button::with_label("Exportar animación GIF…");
    root.append(&export);
    let export_status = Label::builder().wrap(true).build();
    root.append(&export_status);
    export.connect_clicked(glib::clone!(
        #[strong]
        export_busy,
        #[strong]
        visible,
        #[weak]
        kind,
        #[weak]
        export_status,
        move |button| {
            if export_busy.get() {
                return;
            }
            let export_busy = export_busy.clone();
            let snapshots = visible.borrow().clone();
            if snapshots.is_empty() {
                export_status.set_text("No hay registros para exportar.");
                return;
            }
            let selected_kind = kind.selected() as usize;
            let dialog = gtk::FileDialog::builder()
                .title("Guardar evolución corporal")
                .initial_name("bosu-evolucion.gif")
                .build();
            let parent = button.root().and_downcast::<gtk::Window>();
            let button = button.clone();
            let status = export_status.clone();
            dialog.save(
                parent.as_ref(),
                gtk::gio::Cancellable::NONE,
                move |result| {
                    let file = match result {
                        Ok(f) => f,
                        Err(_) => return,
                    };
                    let Some(path) = file.path() else {
                        status.set_text("Elige una carpeta local para guardar el GIF.");
                        return;
                    };
                    if export_busy.replace(true) {
                        return;
                    }
                    button.set_sensitive(false);
                    status.set_text("Exportando animación…");
                    let (sender, receiver) = std::sync::mpsc::channel();
                    std::thread::spawn(move || {
                        let result =
                            body_gif::export(&path, &snapshots, selected_kind).map(|_| path);
                        let _ = sender.send(result);
                    });
                    let weak_button = button.downgrade();
                    let weak_status = status.downgrade();
                    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                        let (Some(button), Some(status)) =
                            (weak_button.upgrade(), weak_status.upgrade())
                        else {
                            return glib::ControlFlow::Break;
                        };
                        match receiver.try_recv() {
                            Ok(result) => {
                                export_busy.set(false);
                                button.set_sensitive(true);
                                match result {
                                    Ok(path) => status
                                        .set_text(&format!("GIF guardado en {}", path.display())),
                                    Err(e) => status.set_text(&format!("No se pudo exportar: {e}")),
                                };
                                glib::ControlFlow::Break
                            }
                            Err(std::sync::mpsc::TryRecvError::Empty) => {
                                glib::ControlFlow::Continue
                            }
                            Err(_) => {
                                export_busy.set(false);
                                button.set_sensitive(true);
                                status.set_text("La exportación se interrumpió.");
                                glib::ControlFlow::Break
                            }
                        }
                    });
                },
            );
        }
    ));
    let refresh: Rc<dyn Fn()> = Rc::new(glib::clone!(
        #[strong]
        export_busy,
        #[strong]
        history,
        #[strong]
        visible,
        #[strong]
        index,
        #[strong]
        update,
        #[weak]
        timeline,
        #[weak]
        range,
        #[weak]
        moment,
        #[weak]
        previous,
        #[weak]
        next,
        #[weak]
        play,
        #[weak]
        export,
        move || {
            let days = match range.selected() {
                1 => Some(7),
                2 => Some(30),
                3 => Some(90),
                _ => None,
            };
            *visible.borrow_mut() = frames(&history.borrow(), days, moment.selected());
            let len = visible.borrow().len();
            index.set(len.saturating_sub(1));
            play.set_active(false);
            play.set_sensitive(len > 1);
            previous.set_sensitive(len > 1);
            next.set_sensitive(len > 1);
            export.set_sensitive(len > 0 && !export_busy.get());
            timeline.set_sensitive(len > 1);
            timeline.set_range(0., len.saturating_sub(1).max(1) as f64);
            timeline.set_value(index.get() as f64);
            update();
        }
    ));
    range.connect_selected_notify(glib::clone!(
        #[strong]
        refresh,
        move |_| refresh()
    ));
    moment.connect_selected_notify(glib::clone!(
        #[strong]
        refresh,
        move |_| refresh()
    ));
    refresh();
    Panel {
        widget: root,
        refresh,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn periods_include_only_recorded_dates_inside_the_window() {
        let today = chrono::Local::now().date_naive();
        let base: Snapshot =
            serde_json::from_str(r#"{"date":"","points":[],"map_version":1}"#).unwrap();
        let records: Vec<_> = [-8, -6, 0, 1]
            .iter()
            .map(|offset| {
                let mut s = base.clone();
                s.date = (today + chrono::Duration::days(*offset))
                    .format("%Y-%m-%d 10:00:00")
                    .to_string();
                s
            })
            .collect();
        assert_eq!(frames(&records, Some(7), 0).len(), 2);
        assert_eq!(frames(&records, None, 0).len(), 4);
    }
    #[test]
    fn timeline_is_sorted_and_filters_moments_without_inventing_zero() {
        let a: Snapshot = serde_json::from_str(
            r#"{"date":"2026-10-02 11:00:00","points":[],"map_version":1,"moment":"after"}"#,
        )
        .unwrap();
        let mut b = a.clone();
        b.date = "2026-10-01 11:00:00".into();
        b.moment = "before".into();
        let mut old = b.clone();
        old.map_version = 0;
        let all = vec![a, b, old];
        assert_eq!(frames(&all, None, 0).len(), 2);
        assert!(frames(&all, None, 0)[0].date.starts_with("2026-10-01"));
        assert_eq!(frames(&all, None, 2).len(), 1);
        assert_eq!(maximum(&all[0], 0, None), None);
    }
}
