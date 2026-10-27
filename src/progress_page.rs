use crate::{models::SessionState, storage::Repository};
use adw::prelude::*;
use chrono::{Datelike, Duration, NaiveDate};
use gtk::{Box, Label};
use std::collections::BTreeSet;
pub fn populate(root: &Box, repo: &Repository) {
    while let Some(child) = root.first_child() {
        root.remove(&child);
    }
    let data = repo.snapshot();
    let today = chrono::Local::now().date_naive();
    let monday = today - Duration::days(i64::from(today.weekday().num_days_from_monday()));
    let since = today - Duration::days(29);
    let goals = adw::PreferencesGroup::builder()
        .title(crate::i18n::tr("Meta semanal"))
        .build();
    for goal in crate::weekly_goal::goals(&data.config, &data.sessions, monday) {
        let row = adw::ActionRow::builder()
            .use_markup(false)
            .title(&goal.name)
            .subtitle(format!("{}/{}", goal.completed, goal.target))
            .tooltip_text(&goal.description)
            .build();
        let pills = Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(4)
            .valign(gtk::Align::Center)
            .build();
        for i in 0..goal.target {
            let pill = Box::builder().css_classes(["goal-pill"]).build();
            if (i as usize) < goal.completed {
                pill.add_css_class("done");
            }
            pills.append(&pill);
        }
        row.add_suffix(&pills);
        goals.add(&row);
    }
    root.append(&goals);
    let recent: Vec<_> = data
        .sessions
        .iter()
        .filter(|s| {
            NaiveDate::parse_from_str(&s.fecha, "%Y-%m-%d").is_ok_and(|d| d >= since && d <= today)
        })
        .collect();
    let active = recent
        .iter()
        .filter(|s| s.has_activity())
        .map(|s| &s.fecha)
        .collect::<BTreeSet<_>>()
        .len();
    let pain: Vec<_> = recent.iter().filter_map(|s| s.dolor).collect();
    let effort: Vec<_> = recent.iter().filter_map(|s| s.esfuerzo).collect();
    let grid = gtk::Grid::builder()
        .column_spacing(12)
        .row_spacing(12)
        .column_homogeneous(true)
        .build();
    for (index, (title, value, tooltip, icon)) in [
        (
            "registros",
            data.sessions.len().to_string(),
            crate::i18n::tr("Sesiones, sensaciones y descansos guardados"),
            "calendar-alt-symbolic",
        ),
        (
            "dolor promedio",
            compact_average(&pain),
            average(&pain),
            "non-emergency-healthcare-symbolic",
        ),
        (
            "días activos",
            active.to_string(),
            crate::i18n::tr("Últimos 30 días · incluye sesiones parciales"),
            "emblem-ok-symbolic",
        ),
        (
            "esfuerzo promedio",
            compact_average(&effort),
            average(&effort),
            "charge-symbolic",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let card = Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .css_classes(["stat-card"])
            .tooltip_text(crate::i18n::tr(&tooltip))
            .build();
        let heading = Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .build();
        heading.append(
            &Label::builder()
                .label(format!(
                    "<span size='xx-large' weight='bold'>{value}</span>"
                ))
                .use_markup(true)
                .halign(gtk::Align::Start)
                .build(),
        );
        heading.append(&gtk::Image::from_icon_name(icon));
        card.append(&heading);
        card.append(
            &Label::builder()
                .label(crate::i18n::tr(title))
                .xalign(0.)
                .css_classes(["dim-label"])
                .build(),
        );
        grid.attach(&card, (index % 2) as i32, (index / 2) as i32, 1, 1);
    }
    root.append(&grid);
    root.append(&section("Esta semana"));
    let week = Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .halign(gtk::Align::Center)
        .build();
    for (i, name) in ["Lun", "Mar", "Mié", "Jue", "Vie", "Sáb", "Dom"]
        .iter()
        .enumerate()
    {
        let date = monday + Duration::days(i as i64);
        let key = date.format("%Y-%m-%d").to_string();
        let day: Vec<_> = data.sessions.iter().filter(|s| s.fecha == key).collect();
        let activity = day.iter().any(|s| s.has_activity());
        let rest = day.iter().any(|s| s.estado == SessionState::Rest);
        let circle = Label::builder()
            .label(if activity {
                "✓"
            } else if rest {
                "◌"
            } else {
                ""
            })
            .css_classes(["circle-box"])
            .tooltip_text(format!("{key} · {}", day.len()))
            .build();
        if activity {
            circle.add_css_class("done");
        }
        let column = Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .build();
        column.append(&circle);
        column.append(
            &Label::builder()
                .label(crate::i18n::tr(name))
                .css_classes(["dim-label"])
                .build(),
        );
        week.append(&column);
    }
    root.append(&week);
    root.append(&section("Últimas cinco semanas"));
    let heatmap = gtk::Grid::builder()
        .column_spacing(4)
        .row_spacing(4)
        .halign(gtk::Align::Center)
        .build();
    for i in 0..35 {
        let date = today - Duration::days(34 - i);
        let key = date.format("%Y-%m-%d").to_string();
        let day: Vec<_> = data.sessions.iter().filter(|s| s.fecha == key).collect();
        let activity = day.iter().any(|s| s.has_activity());
        let rest = day.iter().any(|s| s.estado == SessionState::Rest);
        let completed: usize = day
            .iter()
            .filter(|s| s.has_activity())
            .map(|s| s.ejercicios_completados)
            .sum();
        let total: usize = day
            .iter()
            .filter(|s| s.has_activity())
            .map(|s| s.ejercicios_totales)
            .sum();
        let fraction = if total > 0 {
            completed as f64 / total as f64
        } else {
            0.
        };
        let class = if !activity {
            "heatmap-0"
        } else if fraction > 0.8 {
            "heatmap-4"
        } else if fraction > 0.5 {
            "heatmap-3"
        } else if fraction > 0.2 {
            "heatmap-2"
        } else {
            "heatmap-1"
        };
        let desc = crate::i18n::tr(&format!(
            "{key} · {} registros · {} con actividad",
            day.len(),
            day.iter().filter(|s| s.has_activity()).count()
        ));
        let cell = Label::builder()
            .label(date.day().to_string())
            .css_classes(["heatmap-box", class])
            .tooltip_text(&desc)
            .build();
        if rest && !activity {
            cell.add_css_class("heatmap-rest");
        }
        cell.update_property(&[gtk::accessible::Property::Label(&desc)]);
        heatmap.attach(&cell, (i % 7) as i32, (i / 7) as i32, 1, 1);
    }
    root.append(&heatmap);
    root.append(&section("Dolor · últimos registros"));
    let bars = Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(10)
        .halign(gtk::Align::Center)
        .height_request(125)
        .build();
    let mut measured: Vec<_> = data.sessions.iter().filter(|s| s.dolor.is_some()).collect();
    measured.sort_by(|a, b| {
        a.fecha
            .cmp(&b.fecha)
            .then_with(|| a.registrado_en.cmp(&b.registrado_en))
    });
    for session in measured.iter().skip(measured.len().saturating_sub(7)) {
        let pain = session.dolor.unwrap_or(0.);
        let desc = format!("{} · {pain}/10", session.fecha);
        let column = Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .valign(gtk::Align::End)
            .tooltip_text(&desc)
            .build();
        let bar = Box::builder()
            .height_request((pain * 10.).max(2.) as i32)
            .css_classes(["bar-chart-bar"])
            .build();
        column.append(&bar);
        column.append(&Label::new(Some(&format!("{pain:.0}"))));
        column.update_property(&[gtk::accessible::Property::Label(&desc)]);
        bars.append(&column);
    }
    if measured.is_empty() {
        bars.append(
            &Label::builder()
                .label(crate::i18n::tr("Sin mediciones"))
                .css_classes(["dim-label"])
                .build(),
        );
    }
    root.append(&bars);
    let history = adw::PreferencesGroup::builder()
        .title(crate::i18n::tr("Registros recientes"))
        .build();
    let mut ordered: Vec<_> = data.sessions.iter().collect();
    ordered.sort_by(|a, b| {
        b.fecha
            .cmp(&a.fecha)
            .then_with(|| b.registrado_en.cmp(&a.registrado_en))
    });
    for session in ordered.into_iter().take(10) {
        let row = adw::ExpanderRow::builder()
            .use_markup(false)
            .title(crate::i18n::tr(&format!(
                "{} · {}",
                session.fecha,
                session.estado.label()
            )))
            .subtitle(crate::i18n::tr(&format!(
                "{} · {}/{} ejercicios",
                session.rutina_nombre, session.ejercicios_completados, session.ejercicios_totales
            )))
            .subtitle_lines(2)
            .build();
        if session.estado == SessionState::Legacy {
            row.add_row(&adw::ActionRow::builder().use_markup(false).title(crate::i18n::tr("Medición de una versión anterior")).subtitle(crate::i18n::tr(&format!("Molestia/esfuerzo: {}. Se conserva el valor original; no se incluye en los promedios de dolor.",session.nivel_molestia))).build());
        }
        if !session.notas.is_empty() {
            row.add_row(
                &adw::ActionRow::builder()
                    .use_markup(false)
                    .title(crate::i18n::tr("Notas"))
                    .subtitle(&session.notas)
                    .subtitle_lines(0)
                    .build(),
            );
        }
        for e in &session.ejercicios {
            if e.estado != crate::models::ExerciseState::Pending || !e.realizado.is_empty() {
                row.add_row(
                    &adw::ActionRow::builder()
                        .use_markup(false)
                        .title(&e.nombre)
                        .subtitle(crate::i18n::tr(&format!("{} · {}", e.realizado, e.motivo)))
                        .subtitle_lines(0)
                        .build(),
                );
            }
        }
        let maps = format!(
            "Mapa antes: {} · mapa después: {}",
            if session.mapa_antes.is_some() {
                "registrado"
            } else {
                "sin datos"
            },
            if session.mapa_despues.is_some() {
                "registrado"
            } else {
                "sin datos"
            }
        );
        row.add_row(
            &adw::ActionRow::builder()
                .use_markup(false)
                .title(crate::i18n::tr("Registro corporal vinculado"))
                .subtitle(maps)
                .build(),
        );
        history.add(&row);
    }
    root.append(&history);
}
fn section(title: &str) -> Label {
    Label::builder()
        .label(crate::i18n::tr(title))
        .css_classes(["heading", "dim-label"])
        .xalign(0.)
        .build()
}
fn compact_average(values: &[f64]) -> String {
    if values.is_empty() {
        "—".into()
    } else {
        format!("{:.1}", values.iter().sum::<f64>() / values.len() as f64)
    }
}
fn average(values: &[f64]) -> String {
    if values.is_empty() {
        "Sin mediciones".into()
    } else {
        format!(
            "{:.1}/10 · {} mediciones",
            values.iter().sum::<f64>() / values.len() as f64,
            values.len()
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_data_is_not_zero() {
        assert_eq!(average(&[]), "Sin mediciones");
        assert_eq!(average(&[0.]), "0.0/10 · 1 mediciones");
    }
}
