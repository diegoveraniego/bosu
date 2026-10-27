use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegistroSesion {
    pub fecha: String,
    #[serde(default)]
    pub nivel_molestia: f64,
    pub notas: String,
    pub ejercicios_completados: usize,
    pub ejercicios_totales: usize,
    #[serde(default)]
    pub rutina_id: Option<String>,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub registrado_en: String,
    #[serde(default)]
    pub rutina_nombre: String,
    #[serde(default)]
    pub estado: SessionState,
    #[serde(default)]
    pub dolor: Option<f64>,
    #[serde(default)]
    pub esfuerzo: Option<f64>,
    #[serde(default)]
    pub ejercicios: Vec<ExerciseRecord>,
    #[serde(default)]
    pub mapa_antes: Option<crate::body_map::Snapshot>,
    #[serde(default)]
    pub mapa_despues: Option<crate::body_map::Snapshot>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Ejercicio {
    pub nombre: String,
    pub repeticiones: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Categoria {
    pub titulo: String,
    pub css_class: String,
    pub ejercicios: Vec<Ejercicio>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Rutina {
    pub id: String,
    pub nombre: String,
    pub meta_semanal: u32,
    pub secciones: Vec<Categoria>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Configuracion {
    pub rutinas: Vec<Rutina>,
    pub ultima_rutina_seleccionada: Option<String>,
}

impl Default for Configuracion {
    fn default() -> Self {
        Self {
            rutinas: vec![Rutina {
                id: "default_rutina_1".to_string(),
                nombre: "Sesión Kine Full".to_string(),
                meta_semanal: 3,
                secciones: vec![
                    Categoria {
                        titulo: "Fase 1: Activación y Movilidad".to_string(),
                        css_class: "group-blue".to_string(),
                        ejercicios: vec![
                            Ejercicio {
                                nombre: "Hundir cabeza y pera en almohada".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                            Ejercicio {
                                nombre: "Hundir cabeza en almohada".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                            Ejercicio {
                                nombre: "Pegado a pared brazos en T".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                            Ejercicio {
                                nombre: "Estiramiento rodillo brazos en pared".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                            Ejercicio {
                                nombre: "Estiramientos en colchoneta (yoga)".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                        ],
                    },
                    Categoria {
                        titulo: "Fase 2: Fortalecimiento Escapular".to_string(),
                        css_class: "group-green".to_string(),
                        ejercicios: vec![
                            Ejercicio {
                                nombre: "Bandita en Y".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                            Ejercicio {
                                nombre: "Bandita en Y hacia abajo".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                            Ejercicio {
                                nombre: "Bandita hacia el piso en T".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                            Ejercicio {
                                nombre: "Bandita cruzada".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                            Ejercicio {
                                nombre: "Bandita cruzada con mano contr. arriba".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                            Ejercicio {
                                nombre: "Levantar mancuernas de 2kg".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                            Ejercicio {
                                nombre: "Superman".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                        ],
                    },
                    Categoria {
                        titulo: "Fase 3: Estabilidad de Pie".to_string(),
                        css_class: "group-purple".to_string(),
                        ejercicios: vec![
                            Ejercicio {
                                nombre: "Bandita hacia el pecho".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                            Ejercicio {
                                nombre: "Banda hacia abajo con brazos rectos".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                            Ejercicio {
                                nombre: "Banda hacia abajo (brazo en ángulo recto)".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                            Ejercicio {
                                nombre: "Bandita ángulo recto int. (por brazo)".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                            Ejercicio {
                                nombre: "Bandita ángulo recto ext. (por brazo)".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                        ],
                    },
                    Categoria {
                        titulo: "Fase 4: Resistencia (TRX)".to_string(),
                        css_class: "group-orange".to_string(),
                        ejercicios: vec![
                            Ejercicio {
                                nombre: "Codos pegados al cuerpo, hombros relajados".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                            Ejercicio {
                                nombre: "Codos separados (T), hombros relajados".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                            Ejercicio {
                                nombre: "Estiramiento rodillo lateral".to_string(),
                                repeticiones: "10x3".to_string(),
                            },
                        ],
                    },
                ],
            }],
            ultima_rutina_seleccionada: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    #[default]
    Legacy,
    Partial,
    Complete,
    Rest,
    Observation,
}
impl SessionState {
    pub fn label(self) -> String {
        crate::i18n::tr(match self {
            Self::Legacy => "Registro anterior",
            Self::Partial => "Sesión parcial",
            Self::Complete => "Sesión completa",
            Self::Rest => "Descanso",
            Self::Observation => "Registro de sensaciones",
        })
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExerciseState {
    #[default]
    Pending,
    Done,
    Skipped,
}
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ExerciseRecord {
    pub id: String,
    pub categoria: String,
    pub nombre: String,
    pub planeado: String,
    #[serde(default)]
    pub realizado: String,
    #[serde(default)]
    pub motivo: String,
    #[serde(default)]
    pub estado: ExerciseState,
}
impl RegistroSesion {
    pub fn has_activity(&self) -> bool {
        self.estado != SessionState::Rest
            && (self.ejercicios_completados > 0
                || self
                    .ejercicios
                    .iter()
                    .any(|e| e.estado != ExerciseState::Skipped && !e.realizado.trim().is_empty()))
    }
    pub fn prepare(&mut self) {
        self.ejercicios_totales = self.ejercicios.len();
        self.ejercicios_completados = self
            .ejercicios
            .iter()
            .filter(|e| e.estado == ExerciseState::Done)
            .count();
        if self.estado != SessionState::Rest {
            self.estado = if self.ejercicios_totales > 0
                && self.ejercicios_totales == self.ejercicios_completados
            {
                SessionState::Complete
            } else if self.has_activity() {
                SessionState::Partial
            } else {
                SessionState::Observation
            };
        }
        self.nivel_molestia = self.dolor.unwrap_or(0.);
    }
    pub fn new(routine: Option<&Rutina>) -> Self {
        let now = chrono::Local::now();
        Self {
            fecha: now.format("%Y-%m-%d").to_string(),
            nivel_molestia: 0.,
            notas: String::new(),
            ejercicios_completados: 0,
            ejercicios_totales: 0,
            rutina_id: routine.map(|r| r.id.clone()),
            id: format!("session-{}", gtk::glib::uuid_string_random()),
            registrado_en: now.to_rfc3339(),
            rutina_nombre: routine
                .map(|r| r.nombre.clone())
                .unwrap_or_else(|| "Descanso".into()),
            estado: if routine.is_some() {
                SessionState::Observation
            } else {
                SessionState::Rest
            },
            dolor: None,
            esfuerzo: None,
            ejercicios: routine
                .map(|r| {
                    r.secciones
                        .iter()
                        .enumerate()
                        .flat_map(|(ci, c)| {
                            c.ejercicios
                                .iter()
                                .enumerate()
                                .map(move |(ei, e)| ExerciseRecord {
                                    id: format!("{ci}:{ei}"),
                                    categoria: c.titulo.clone(),
                                    nombre: e.nombre.clone(),
                                    planeado: e.repeticiones.clone(),
                                    ..Default::default()
                                })
                        })
                        .collect()
                })
                .unwrap_or_default(),
            mapa_antes: None,
            mapa_despues: None,
        }
    }
}
