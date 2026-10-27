use crate::{Configuracion, RegistroSesion};
use chrono::{Duration, NaiveDate};
use std::collections::BTreeSet;
pub struct Goal {
    pub name: String,
    pub description: String,
    pub target: u32,
    pub completed: usize,
}
pub fn goals(config: &Configuracion, history: &[RegistroSesion], monday: NaiveDate) -> Vec<Goal> {
    let days = |ids: &[&str]| {
        history
            .iter()
            .filter(|s| {
                s.has_activity() && s.rutina_id.as_deref().is_some_and(|id| ids.contains(&id))
            })
            .filter_map(|s| NaiveDate::parse_from_str(&s.fecha, "%Y-%m-%d").ok())
            .filter(|d| *d >= monday && *d < monday + Duration::days(7))
            .collect::<BTreeSet<_>>()
            .len()
    };
    let shared = ["default_rutina_1", "sesion_kine_1", "sesion_kine_2"];
    let mut out = vec![];
    if config
        .rutinas
        .iter()
        .any(|r| shared.contains(&r.id.as_str()))
    {
        out.push(Goal{name:crate::i18n::tr("Días de práctica de kine"),description:crate::i18n::tr("Una sesión parcial también cuenta. Varias sesiones el mismo día suman un día de práctica."),target:config.rutinas.iter().find(|r|r.id==shared[0]).map_or(3,|r|r.meta_semanal),completed:days(&shared)});
    }
    for r in config
        .rutinas
        .iter()
        .filter(|r| !shared.contains(&r.id.as_str()))
    {
        out.push(Goal {
            name: r.nombre.clone(),
            description: crate::i18n::tr("Días con actividad, incluyendo sesiones parciales."),
            target: r.meta_semanal,
            completed: days(&[&r.id]),
        });
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partials_count_and_same_day_is_not_double_counted() {
        let c = Configuracion::default();
        let monday = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
        let mut a = RegistroSesion::new(Some(&c.rutinas[0]));
        a.fecha = "2026-10-26".into();
        a.ejercicios[0].estado = crate::models::ExerciseState::Done;
        a.prepare();
        let mut b = a.clone();
        b.fecha = "2026-10-27".into();
        let mut old = a.clone();
        old.fecha = "2026-10-25".into();
        assert_eq!(goals(&c, &[a.clone(), a, b, old], monday)[0].completed, 2);
    }
}
