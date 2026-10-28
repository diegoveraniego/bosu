use crate::{Categoria, Configuracion, Ejercicio, Rutina};

fn section(title: &str, items: &[(&str, &str)]) -> Categoria {
    Categoria {
        titulo: title.into(),
        css_class: "group-blue".into(),
        ejercicios: items
            .iter()
            .map(|(name, reps)| Ejercicio {
                nombre: (*name).into(),
                repeticiones: (*reps).into(),
            })
            .collect(),
    }
}

fn parts() -> Vec<Rutina> {
    let warmup = section("Calentamiento", &[("Bici o elíptica", "15 minutos")]);
    let stretches = section(
        "Estiramientos · siempre al final",
        &[
            (
                "Oreja al hombro, mirar ese hombro y tirar con el brazo de ese lado",
                "20 alternando izquierda y derecha · 10 s cada vez",
            ),
            (
                "Oreja al hombro, mirar el hombro contrario y tirar con el brazo del lado que miras",
                "20 alternando izquierda y derecha · 10 s cada vez",
            ),
            (
                "Brazo en barra vertical: mover pecho y mirada al lado contrario, arriba, al medio y abajo",
                "10 s en cada posición",
            ),
        ],
    );
    vec![
        Rutina {
            id: "sesion_kine_1".into(),
            nombre: "Sesión Kine · Parte 1: Pared y suelo".into(),
            meta_semanal: 1,
            secciones: vec![
                warmup.clone(),
                section(
                    "Limpiar pared · opcional si te sientes fuerte: mantener 5 s",
                    &[
                        ("Limpiar hacia adelante y arriba en V", "50 repeticiones"),
                        ("Limpiar en círculos hacia adentro", "50 repeticiones"),
                        ("Limpiar en círculos hacia afuera", "50 repeticiones"),
                        (
                            "Limpiar hacia izquierda y derecha con un brazo, alternando",
                            "100 totales · 50 por lado",
                        ),
                    ],
                ),
                section(
                    "Acostado · opcional: mantener 5 s cada repetición, excepto saludo al sol",
                    &[
                        ("Levantar escápulas mirando al piso", "50 repeticiones"),
                        (
                            "Levantar brazos hacia atrás mirando al piso",
                            "50 repeticiones",
                        ),
                        (
                            "Superman de calentamiento de escalada, boca abajo: manos hacia adelante y atrás",
                            "50 repeticiones",
                        ),
                        ("Saludo al sol", "50 repeticiones"),
                    ],
                ),
                stretches.clone(),
            ],
        },
        Rutina {
            id: "sesion_kine_2".into(),
            nombre: "Sesión Kine · Parte 2: TRX y barra".into(),
            meta_semanal: 1,
            secciones: vec![
                warmup,
                section(
                    "TRX",
                    &[
                        ("Brazos horizontales", "50 repeticiones"),
                        ("Brazos con codos pegados al cuerpo", "50 repeticiones"),
                        ("Manos mirando hacia arriba", "50 repeticiones"),
                        (
                            "Colgar de un brazo y llevar la mano contraria a la mano que se agarra",
                            "50 por lado · izquierda y derecha",
                        ),
                        (
                            "De espaldas al TRX: ir de puntas con ambos brazos hacia arriba y bajar",
                            "50 repeticiones",
                        ),
                    ],
                ),
                section(
                    "Barra · con apoyo de pies, sin colgar",
                    &[
                        (
                            "Juntar escápulas en tres posiciones de dominada, apoyado en un piso",
                            "50 en cada posición",
                        ),
                        (
                            "Comienzo de dominada: coordinar escápulas con ayuda de pies, en tres agarres",
                            "50 en cada agarre · sin colgar",
                        ),
                    ],
                ),
                stretches,
            ],
        },
    ]
}

fn merge_sections(first: &mut Rutina, second: Rutina) {
    let mut stretches = Vec::new();
    let mut sections = Vec::new();
    for section in first.secciones.drain(..).chain(second.secciones) {
        let target = if section.titulo.starts_with("Estiramientos") {
            &mut stretches
        } else {
            &mut sections
        };
        if !target.iter().any(|existing: &Categoria| {
            existing.titulo == section.titulo
                && existing.ejercicios.len() == section.ejercicios.len()
                && existing
                    .ejercicios
                    .iter()
                    .zip(&section.ejercicios)
                    .all(|(a, b)| a.nombre == b.nombre && a.repeticiones == b.repeticiones)
        }) {
            target.push(section);
        }
    }
    sections.extend(stretches);
    first.secciones = sections;
}

pub fn routines() -> Vec<Rutina> {
    let mut parts = parts().into_iter();
    let mut routine = parts.next().unwrap();
    merge_sections(&mut routine, parts.next().unwrap());
    routine.nombre = "Sesión Kine · Pared, suelo, TRX y barra".into();
    vec![routine]
}

pub fn add_missing(config: &mut Configuracion) -> bool {
    let mut changed = false;
    for routine in &mut config.rutinas {
        if routine.id == "default_rutina_1"
            && matches!(routine.nombre.as_str(), "Rutina Principal" | "Rutina 1")
        {
            routine.nombre = "Sesión Kine Full".into();
            changed = true;
        }
    }

    let had_second = config.rutinas.iter().any(|r| r.id == "sesion_kine_2");
    if let Some(index) = config.rutinas.iter().position(|r| r.id == "sesion_kine_2") {
        let second = config.rutinas.remove(index);
        if let Some(first) = config.rutinas.iter_mut().find(|r| r.id == "sesion_kine_1") {
            merge_sections(first, second);
        } else {
            let mut first = parts().remove(0);
            merge_sections(&mut first, second);
            config.rutinas.push(first);
        }
        changed = true;
    }
    if config.ultima_rutina_seleccionada.as_deref() == Some("sesion_kine_2") {
        config.ultima_rutina_seleccionada = Some("sesion_kine_1".into());
        changed = true;
    }
    for routine in routines() {
        if let Some(first) = config.rutinas.iter_mut().find(|r| r.id == routine.id) {
            if matches!(
                first.nombre.as_str(),
                "Sesión Kine · Pared y suelo" | "Sesión Kine · Parte 1: Pared y suelo"
            ) {
                // An installation with only the first part also needs the second.
                if !had_second {
                    merge_sections(first, parts().remove(1));
                }
                first.nombre = routine.nombre;
                changed = true;
            }
        } else {
            config.rutinas.push(routine);
            changed = true;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn combines_existing_parts_preserving_edits_and_selection() {
        let mut config = Configuracion::default();
        config.rutinas.extend(parts());
        config.rutinas[2].secciones[1].ejercicios[0].repeticiones = "12 personalizadas".into();
        config.ultima_rutina_seleccionada = Some("sesion_kine_2".into());
        assert!(add_missing(&mut config));
        assert_eq!(config.rutinas.len(), 2);
        let merged = &config.rutinas[1];
        assert_eq!(merged.secciones.len(), 6);
        assert_eq!(merged.secciones[0].titulo, "Calentamiento");
        assert!(merged.secciones[5].titulo.starts_with("Estiramientos"));
        assert_eq!(
            merged.secciones[3].ejercicios[0].repeticiones,
            "12 personalizadas"
        );
        assert_eq!(
            config.ultima_rutina_seleccionada.as_deref(),
            Some("sesion_kine_1")
        );
        assert!(!add_missing(&mut config));
    }

    #[test]
    fn migration_preserves_customization_and_is_idempotent() {
        let mut config = Configuracion::default();
        assert!(add_missing(&mut config));
        config.rutinas.last_mut().unwrap().nombre = "Personalizada".into();
        assert!(!add_missing(&mut config));
        assert_eq!(config.rutinas.last().unwrap().nombre, "Personalizada");
        for routine in routines() {
            assert!(
                routine
                    .secciones
                    .last()
                    .unwrap()
                    .titulo
                    .starts_with("Estiramientos")
            );
        }
    }
}
