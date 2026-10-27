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
    root.append(
        &Label::builder()
            .label(crate::i18n::tr("Tu proceso"))
            .css_classes(["title-1"])
            .xalign(0.)
            .build(),
    );
    root.append(&Label::builder().label(crate::i18n::tr("Las sesiones parciales cuentan como actividad. Los días sin registro quedan sin datos; descansar no borra tu avance.")).wrap(true).xalign(0.).build());
    let goals = adw::PreferencesGroup::builder()
        .title(crate::i18n::tr("Esta semana · días de práctica"))
        .build();
    for goal in crate::weekly_goal::goals(&data.config, &data.sessions, monday) {
        let row = adw::ActionRow::builder()
            .use_markup(false)
            .title(&goal.name)
            .subtitle(crate::i18n::tr(&format!(
                "{}/{} días · {}",
                goal.completed, goal.target, goal.description
            )))
            .subtitle_lines(3)
            .build();
        let progress = gtk::ProgressBar::builder()
            .fraction(if goal.target > 0 {
                (goal.completed as f64 / f64::from(goal.target)).min(1.)
            } else {
                0.
            })
            .width_request(64)
            .valign(gtk::Align::Center)
            .build();
        row.add_suffix(&progress);
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
    let partial = recent
        .iter()
        .filter(|s| {
            s.estado == SessionState::Partial
                || (s.ejercicios_completados > 0 && s.ejercicios_completados < s.ejercicios_totales)
        })
        .count();
    let rests = recent
        .iter()
        .filter(|s| s.estado == SessionState::Rest)
        .count();
    let pain: Vec<_> = recent.iter().filter_map(|s| s.dolor).collect();
    let effort: Vec<_> = recent.iter().filter_map(|s| s.esfuerzo).collect();
    let group=adw::PreferencesGroup::builder().title(crate::i18n::tr("Últimos 30 días")).description(crate::i18n::tr("Promedios calculados solo con mediciones explícitas. Dolor y esfuerzo antiguos mezclados no se reinterpretan.")).build();
    for (title, value) in [
        ("Días con actividad", active.to_string()),
        ("Sesiones parciales", partial.to_string()),
        ("Descansos registrados", rests.to_string()),
        ("Dolor registrado", average(&pain)),
        ("Esfuerzo registrado", average(&effort)),
    ] {
        group.add(
            &adw::ActionRow::builder()
                .use_markup(false)
                .title(crate::i18n::tr(title))
                .subtitle(crate::i18n::tr(&value))
                .build(),
        );
    }
    root.append(&group);
    let calendar = adw::PreferencesGroup::builder()
        .title(crate::i18n::tr("Últimas cinco semanas"))
        .description(crate::i18n::tr(
            "Cada celda resume todos los registros de ese día.",
        ))
        .build();
    let grid = gtk::Grid::builder()
        .column_spacing(5)
        .row_spacing(5)
        .halign(gtk::Align::Center)
        .build();
    for i in 0..35 {
        let date = today - Duration::days(34 - i);
        let key = date.format("%Y-%m-%d").to_string();
        let day: Vec<_> = data.sessions.iter().filter(|s| s.fecha == key).collect();
        let activity = day.iter().any(|s| s.has_activity());
        let rest = day.iter().any(|s| s.estado == SessionState::Rest);
        let label = Label::builder()
            .label(format!(
                "{}{}",
                date.day(),
                if activity {
                    " ●"
                } else if rest {
                    " ◌"
                } else {
                    ""
                }
            ))
            .width_request(30)
            .height_request(30)
            .css_classes(["calendar-day"])
            .build();
        if activity {
            label.add_css_class("activity-day");
        } else if rest {
            label.add_css_class("rest-day");
        }
        let desc = format!(
            "{key} · {} registros · {} con actividad{}",
            day.len(),
            day.iter().filter(|s| s.has_activity()).count(),
            if rest { " · descanso registrado" } else { "" }
        );
        label.set_tooltip_text(Some(&desc));
        label.update_property(&[gtk::accessible::Property::Label(&desc)]);
        grid.attach(&label, (i % 7) as i32, (i / 7) as i32, 1, 1);
    }
    calendar.add(&grid);
    root.append(&calendar);
    let history = adw::PreferencesGroup::builder()
        .title(crate::i18n::tr("Registros recientes"))
        .build();
    if data.sessions.is_empty() {
        root.append(
            &adw::StatusPage::builder()
                .icon_name("document-edit-symbolic")
                .title(crate::i18n::tr("Tu proceso empieza con un registro"))
                .description(crate::i18n::tr(
                    "Puedes guardar una sesión parcial, sensaciones o un día de descanso.",
                ))
                .build(),
        );
    }
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
