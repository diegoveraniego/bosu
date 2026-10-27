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

pub fn routines() -> Vec<Rutina> {
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
            nombre: "Sesión Kine · Pared y suelo".into(),
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
            nombre: "Sesión Kine · TRX y barra".into(),
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

pub fn add_missing(config: &mut Configuracion) -> bool {
    let mut changed = false;
    for routine in routines() {
        if !config.rutinas.iter().any(|r| r.id == routine.id) {
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
