//! One validated, atomic store keeps sessions, linked maps and drafts consistent.
use crate::{Configuracion, RegistroSesion, body_map::Snapshot};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    rc::Rc,
};
const VERSION: u32 = 1;
#[derive(Clone, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default)]
    pub reminders: bool,
    #[serde(default = "default_hour")]
    pub reminder_hour: u32,
    #[serde(default)]
    pub reminder_minute: u32,
    #[serde(default)]
    pub reminder_last_date: String,
    #[serde(default)]
    pub snooze_until: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub appearance: String,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            reminders: false,
            reminder_hour: 18,
            reminder_minute: 0,
            reminder_last_date: String::new(),
            snooze_until: String::new(),
            language: String::new(),
            appearance: String::new(),
        }
    }
}
fn default_hour() -> u32 {
    18
}
#[derive(Clone, Serialize, Deserialize)]
pub struct DeletedSession {
    pub position: usize,
    pub session: RegistroSesion,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Store {
    pub version: u32,
    pub config: Configuracion,
    #[serde(default)]
    pub sessions: Vec<RegistroSesion>,
    #[serde(default)]
    pub body: Vec<Snapshot>,
    #[serde(default)]
    pub drafts: BTreeMap<String, RegistroSesion>,
    #[serde(default)]
    pub selected_draft: Option<String>,
    #[serde(default)]
    pub body_draft: Option<Snapshot>,
    #[serde(default)]
    pub preferences: Preferences,
    #[serde(default)]
    pub deleted: Option<DeletedSession>,
}
impl Default for Store {
    fn default() -> Self {
        let mut config = Configuracion::default();
        crate::kine::add_missing(&mut config);
        Self {
            version: VERSION,
            config,
            sessions: vec![],
            body: vec![],
            drafts: BTreeMap::new(),
            selected_draft: None,
            body_draft: None,
            preferences: Preferences::default(),
            deleted: None,
        }
    }
}
impl Store {
    /// Resume today's saved work instead of the empty form left after saving.
    pub fn session_for_day(&self, key: &str, today: &str) -> Option<RegistroSesion> {
        let draft = self.drafts.get(key);
        if let Some(draft) = draft {
            let has_changes = self.sessions.iter().any(|s| s.id == draft.id)
                || draft.fecha != today
                || !draft.notas.trim().is_empty()
                || draft.dolor.is_some()
                || draft.esfuerzo.is_some()
                || draft.mapa_antes.is_some()
                || draft.mapa_despues.is_some()
                || draft.ejercicios.iter().any(|e| {
                    e.estado != crate::models::ExerciseState::Pending
                        || !e.realizado.trim().is_empty()
                        || !e.motivo.trim().is_empty()
                });
            if has_changes {
                return Some(draft.clone());
            }
        }
        self.sessions
            .iter()
            .filter(|s| {
                s.fecha == today
                    && if key == "__rest__" {
                        s.rutina_id.is_none()
                    } else {
                        s.rutina_id.as_deref() == Some(key) && !s.ejercicios.is_empty()
                    }
            })
            .max_by(|a, b| a.registrado_en.cmp(&b.registrado_en))
            .cloned()
            .or_else(|| draft.cloned())
    }
}
#[derive(Serialize, Deserialize)]
struct Backup {
    format: String,
    version: u32,
    data: Store,
}
#[derive(Clone)]
pub struct Repository {
    root: PathBuf,
    data: Rc<RefCell<Store>>,
}
fn err(path: &Path, e: impl std::fmt::Display) -> String {
    format!("{}: {e}", path.display())
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Ruta sin carpeta")?;
    fs::create_dir_all(parent).map_err(|e| err(parent, e))?;
    let temp = parent.join(format!(".bosu-{}.tmp", gtk::glib::uuid_string_random()));
    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temp).map_err(|e| err(&temp, e))?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| err(&temp, e))?;
        drop(file);
        fs::rename(&temp, path).map_err(|e| err(path, e))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
fn read_optional<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>, String> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| err(path, e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(err(path, e)),
    }
}
impl Repository {
    pub fn open(root: PathBuf) -> Result<Self, String> {
        let path = root.join("bosu_data.json");
        let data = if let Some(data) = read_optional::<Store>(&path)? {
            validate(&data)?;
            data
        } else {
            let mut data = Store::default();
            if let Some(config) = read_optional(&root.join("bosu_config.json"))? {
                data.config = config;
                crate::kine::add_missing(&mut data.config);
            }
            let old = root.join("bosu_history.jsonl");
            match fs::read_to_string(&old) {
                Ok(text) => {
                    for (index, line) in text
                        .lines()
                        .enumerate()
                        .filter(|(_, s)| !s.trim().is_empty())
                    {
                        let mut session: RegistroSesion = serde_json::from_str(line)
                            .map_err(|e| format!("{} · línea {}: {e}", old.display(), index + 1))?;
                        if session.id.is_empty() {
                            session.id = format!("legacy-{index}-{}", session.fecha);
                        }
                        if session.rutina_nombre.is_empty() {
                            session.rutina_nombre = data
                                .config
                                .rutinas
                                .iter()
                                .find(|r| Some(r.id.as_str()) == session.rutina_id.as_deref())
                                .map(|r| r.nombre.clone())
                                .unwrap_or_else(|| "Rutina anterior".into());
                        }
                        data.sessions.push(session);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(err(&old, e)),
            }
            data.body = read_optional(&root.join("bosu_body.json"))?.unwrap_or_default();
            validate(&data)?;
            atomic_write(
                &path,
                &serde_json::to_vec_pretty(&data).map_err(|e| e.to_string())?,
            )?;
            data
        };
        Ok(Self {
            root,
            data: Rc::new(RefCell::new(data)),
        })
    }
    pub fn recover(root: PathBuf, backup_path: &Path) -> Result<Self, String> {
        let incoming = Self::inspect_backup(backup_path)?;
        let preserved = root
            .join("backups")
            .join(format!("recovery-{}", gtk::glib::uuid_string_random()));
        for name in [
            "bosu_data.json",
            "bosu_config.json",
            "bosu_history.jsonl",
            "bosu_body.json",
        ] {
            let source = root.join(name);
            match fs::read(&source) {
                Ok(bytes) => atomic_write(&preserved.join(name), &bytes)?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(err(&source, error)),
            }
        }
        atomic_write(
            &root.join("bosu_data.json"),
            &serde_json::to_vec_pretty(&incoming).map_err(|e| e.to_string())?,
        )?;
        Self::open(root)
    }
    pub fn snapshot(&self) -> Store {
        self.data.borrow().clone()
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn update(
        &self,
        change: impl FnOnce(&mut Store) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut next = self.snapshot();
        change(&mut next)?;
        validate(&next)?;
        atomic_write(
            &self.root.join("bosu_data.json"),
            &serde_json::to_vec_pretty(&next).map_err(|e| e.to_string())?,
        )?;
        *self.data.borrow_mut() = next;
        Ok(())
    }
    pub fn save_session(&self, key: &str, mut session: RegistroSesion) -> Result<(), String> {
        session.prepare();
        validate_session(&session)?;
        self.update(|data| {
            if let Some(existing) = data.sessions.iter_mut().find(|s| s.id == session.id) {
                *existing = session;
            } else {
                data.sessions.push(session);
            }
            data.drafts.remove(key);
            Ok(())
        })
    }
    pub fn delete_session(&self, id: &str) -> Result<(), String> {
        self.update(|data| {
            let position = data
                .sessions
                .iter()
                .position(|s| s.id == id)
                .ok_or("El registro ya no existe")?;
            let session = data.sessions.remove(position);
            data.deleted = Some(DeletedSession { position, session });
            Ok(())
        })
    }
    pub fn undo_delete_for(&self, id: &str) -> Result<(), String> {
        self.update(|data| {
            if data.deleted.as_ref().is_none_or(|d| d.session.id != id) {
                return Err(
                    "Esa eliminación ya no es la última. No se modificó ningún registro.".into(),
                );
            }
            let deleted = data.deleted.take().unwrap();
            data.sessions
                .insert(deleted.position.min(data.sessions.len()), deleted.session);
            Ok(())
        })
    }
    pub fn undo_delete(&self) -> Result<(), String> {
        self.update(|data| {
            let deleted = data
                .deleted
                .take()
                .ok_or("No hay una eliminación para deshacer")?;
            data.sessions
                .insert(deleted.position.min(data.sessions.len()), deleted.session);
            Ok(())
        })
    }
    pub fn all_maps(&self) -> Vec<Snapshot> {
        let data = self.snapshot();
        let mut maps = data.body;
        for session in data.sessions {
            for (moment, map) in [
                ("before", session.mapa_antes),
                ("after", session.mapa_despues),
            ] {
                if let Some(mut map) = map {
                    map.session_id = Some(session.id.clone());
                    map.moment = moment.into();
                    if !map.date.starts_with(&session.fecha) {
                        map.date = session.fecha.clone();
                    }
                    maps.push(map);
                }
            }
        }
        maps.sort_by(|a, b| a.date.cmp(&b.date));
        maps
    }
    pub fn export(&self, path: &Path) -> Result<(), String> {
        let backup = Backup {
            format: "org.diego.Bosu.backup".into(),
            version: VERSION,
            data: self.snapshot(),
        };
        atomic_write(
            path,
            &serde_json::to_vec_pretty(&backup).map_err(|e| e.to_string())?,
        )
    }
    pub fn export_file(&self, file: &gtk::gio::File) -> Result<(), String> {
        use gtk::prelude::*;
        let backup = Backup {
            format: "org.diego.Bosu.backup".into(),
            version: VERSION,
            data: self.snapshot(),
        };
        let bytes = serde_json::to_vec_pretty(&backup).map_err(|e| e.to_string())?;
        file.replace_contents(
            &bytes,
            None,
            false,
            gtk::gio::FileCreateFlags::PRIVATE | gtk::gio::FileCreateFlags::REPLACE_DESTINATION,
            gtk::gio::Cancellable::NONE,
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
    }
    pub fn inspect_backup(path: &Path) -> Result<Store, String> {
        let backup: Backup = read_optional(path)?.ok_or("No existe el respaldo")?;
        if backup.format != "org.diego.Bosu.backup" || backup.version != VERSION {
            return Err("Formato o versión de respaldo incompatible".into());
        }
        validate(&backup.data)?;
        Ok(backup.data)
    }
    pub fn import(&self, path: &Path) -> Result<PathBuf, String> {
        let next = Self::inspect_backup(path)?;
        let previous = self.root.join("backups").join(format!(
            "antes-importar-{}.json",
            gtk::glib::uuid_string_random()
        ));
        self.export(&previous)?;
        self.update(|data| {
            *data = next;
            Ok(())
        })?;
        Ok(previous)
    }
}
fn validate_map(map: &Snapshot) -> Result<(), String> {
    if map.map_version > 1 {
        return Err("Versión de mapa corporal incompatible".into());
    }
    for p in &map.points {
        if !p.x.is_finite()
            || !p.y.is_finite()
            || !(0.0..=1.0).contains(&p.x)
            || !(0.0..=1.0).contains(&p.y)
            || p.kind >= 6
            || p.intensity.is_some_and(|v| v > 10)
        {
            return Err("Punto corporal con valores inválidos".into());
        }
    }
    Ok(())
}
fn validate_session(s: &RegistroSesion) -> Result<(), String> {
    chrono::NaiveDate::parse_from_str(&s.fecha, "%Y-%m-%d")
        .map_err(|_| "La fecha debe tener el formato AAAA-MM-DD".to_string())?;
    for v in [s.dolor, s.esfuerzo].into_iter().flatten() {
        if !v.is_finite() || !(0.0..=10.0).contains(&v) {
            return Err("Dolor y esfuerzo deben estar entre 0 y 10".into());
        }
    }
    if s.ejercicios_completados > s.ejercicios_totales {
        return Err("Cantidad de ejercicios inconsistente".into());
    }
    for map in [&s.mapa_antes, &s.mapa_despues].into_iter().flatten() {
        validate_map(map)?;
    }
    Ok(())
}
fn validate(data: &Store) -> Result<(), String> {
    if data.version != VERSION {
        return Err("Esta versión de Bosu no puede abrir ese formato de datos".into());
    }
    if data.preferences.reminder_hour > 23 || data.preferences.reminder_minute > 59 {
        return Err("Horario de recordatorio inválido".into());
    }
    let mut ids = std::collections::HashSet::new();
    for r in &data.config.rutinas {
        if r.id.is_empty() || !ids.insert(&r.id) || r.meta_semanal > 7 {
            return Err("La configuración contiene rutinas o metas inválidas".into());
        }
    }
    let mut sessions = std::collections::HashSet::new();
    for s in &data.sessions {
        validate_session(s)?;
        if s.id.is_empty() || !sessions.insert(&s.id) {
            return Err("Identificadores de sesión vacíos o duplicados".into());
        }
    }
    for draft in data.drafts.values() {
        validate_session(draft)?;
    }
    for map in data.body.iter().chain(data.body_draft.iter()) {
        validate_map(map)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn folder() -> PathBuf {
        let p = std::env::temp_dir().join(format!("bosu-test-{}", gtk::glib::uuid_string_random()));
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn successful_migration_preserves_originals_and_legacy_measurements() {
        let root = folder();
        let history = r#"{"fecha":"2026-10-01","nivel_molestia":4.0,"notas":"Anterior","ejercicios_completados":1,"ejercicios_totales":3,"rutina_id":"default_rutina_1"}"#;
        fs::write(root.join("bosu_history.jsonl"), history).unwrap();
        let repo = Repository::open(root.clone()).unwrap();
        let data = repo.snapshot();
        assert_eq!(data.sessions.len(), 1);
        assert_eq!(data.sessions[0].nivel_molestia, 4.);
        assert_eq!(data.sessions[0].dolor, None);
        assert!(data.sessions[0].has_activity());
        assert_eq!(
            fs::read_to_string(root.join("bosu_history.jsonl")).unwrap(),
            history
        );
        assert_eq!(
            Repository::open(root.clone())
                .unwrap()
                .snapshot()
                .sessions
                .len(),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn recovery_keeps_raw_corrupt_data_before_restoring() {
        let root = folder();
        let repo = Repository::open(root.clone()).unwrap();
        let backup = root.join("safe.json");
        repo.export(&backup).unwrap();
        let raw = b"{ broken store";
        fs::write(root.join("bosu_data.json"), raw).unwrap();
        assert!(Repository::open(root.clone()).is_err());
        Repository::recover(root.clone(), &backup).unwrap();
        let preserved = fs::read_dir(root.join("backups"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path()
            .join("bosu_data.json");
        assert_eq!(fs::read(preserved).unwrap(), raw);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn drafts_survive_restart_and_zero_is_an_explicit_measurement() {
        let root = folder();
        let repo = Repository::open(root.clone()).unwrap();
        let mut session = RegistroSesion::new(Some(&repo.snapshot().config.rutinas[0]));
        session.dolor = Some(0.);
        session.esfuerzo = Some(6.);
        session.notas = "Seguir mañana".into();
        repo.update(|d| {
            d.drafts.insert("active".into(), session);
            Ok(())
        })
        .unwrap();
        let restored = Repository::open(root.clone()).unwrap().snapshot().drafts["active"].clone();
        assert_eq!(restored.notas, "Seguir mañana");
        assert_eq!(restored.dolor, Some(0.));
        assert_eq!(restored.esfuerzo, Some(6.));
        assert_eq!(restored.ejercicios_completados, 0);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn corrupt_legacy_data_is_preserved_and_migration_stops() {
        let root = folder();
        let original = b"not json\n";
        fs::write(root.join("bosu_history.jsonl"), original).unwrap();
        assert!(Repository::open(root.clone()).is_err());
        assert_eq!(fs::read(root.join("bosu_history.jsonl")).unwrap(), original);
        assert!(!root.join("bosu_data.json").exists());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn partial_save_is_idempotent_and_undo_restores_maps() {
        let root = folder();
        let repo = Repository::open(root.clone()).unwrap();
        let routine = repo.snapshot().config.rutinas[0].clone();
        let mut session = RegistroSesion::new(Some(&routine));
        session.ejercicios[0].estado = crate::models::ExerciseState::Done;
        session.mapa_antes=Some(serde_json::from_str(r#"{"date":"2026-10-27 12:00:00","map_version":1,"moment":"before","points":[{"x":0.4,"y":0.2,"back":false,"kind":0,"region_id":0,"intensity":5}]}"#).unwrap());
        let id = session.id.clone();
        repo.save_session("draft", session.clone()).unwrap();
        repo.save_session("draft", session).unwrap();
        assert_eq!(repo.snapshot().sessions.len(), 1);
        assert_eq!(
            repo.snapshot().sessions[0].estado,
            crate::models::SessionState::Partial
        );
        assert_eq!(repo.all_maps().len(), 1);
        repo.delete_session(&id).unwrap();
        assert!(repo.snapshot().sessions.is_empty());
        assert!(repo.all_maps().is_empty());
        repo.undo_delete().unwrap();
        assert_eq!(repo.snapshot().sessions[0].id, id);
        assert_eq!(repo.all_maps()[0].session_id.as_deref(), Some(id.as_str()));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn today_resumes_saved_work_and_updates_it_without_duplicates() {
        let root = folder();
        let repo = Repository::open(root.clone()).unwrap();
        let routine = repo.snapshot().config.rutinas[0].clone();
        let mut session = RegistroSesion::new(Some(&routine));
        let today = session.fecha.clone();
        session.ejercicios[0].estado = crate::models::ExerciseState::Done;
        repo.save_session(&routine.id, session.clone()).unwrap();
        // Older versions left an empty draft after saving.
        repo.update(|data| {
            data.drafts
                .insert(routine.id.clone(), RegistroSesion::new(Some(&routine)));
            Ok(())
        })
        .unwrap();
        let reopened = Repository::open(root.clone()).unwrap();
        let mut resumed = reopened
            .snapshot()
            .session_for_day(&routine.id, &today)
            .unwrap();
        assert_eq!(resumed.id, session.id);
        assert_eq!(resumed.ejercicios_completados, 1);
        resumed.ejercicios[1].estado = crate::models::ExerciseState::Done;
        reopened.save_session(&routine.id, resumed.clone()).unwrap();
        assert_eq!(reopened.snapshot().sessions.len(), 1);
        assert_eq!(reopened.snapshot().sessions[0].ejercicios_completados, 2);
        assert_eq!(
            reopened
                .snapshot()
                .session_for_day(&routine.id, &today)
                .unwrap()
                .id,
            session.id
        );
        assert!(
            reopened
                .snapshot()
                .session_for_day(&routine.id, "2099-01-01")
                .is_none()
        );
        // Unchecking every exercise is an edit and must survive reopening too.
        for e in &mut resumed.ejercicios {
            e.estado = crate::models::ExerciseState::Pending;
        }
        resumed.prepare();
        reopened
            .update(|data| {
                data.drafts.insert(routine.id.clone(), resumed);
                Ok(())
            })
            .unwrap();
        assert_eq!(
            reopened
                .snapshot()
                .session_for_day(&routine.id, &today)
                .unwrap()
                .ejercicios_completados,
            0
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn today_preserves_unsaved_work_and_other_routines() {
        let mut data = Store::default();
        let routine = data.config.rutinas[0].clone();
        let mut saved = RegistroSesion::new(Some(&routine));
        let today = saved.fecha.clone();
        saved.ejercicios[0].estado = crate::models::ExerciseState::Done;
        data.sessions.push(saved);
        let mut draft = RegistroSesion::new(Some(&routine));
        draft.notas = "Cambios sin guardar".into();
        data.drafts.insert(routine.id.clone(), draft.clone());
        assert_eq!(
            data.session_for_day(&routine.id, &today).unwrap().id,
            draft.id
        );
        assert!(data.session_for_day("otra-rutina", &today).is_none());
        assert!(data.session_for_day("__rest__", &today).is_none());
    }
    #[test]
    fn backup_roundtrip_and_failed_import_preserve_current_data() {
        let root = folder();
        let repo = Repository::open(root.clone()).unwrap();
        let mut s = RegistroSesion::new(None);
        s.notas = "Descanso".into();
        repo.save_session("rest", s).unwrap();
        let backup = root.join("backup.json");
        repo.export_file(&gtk::gio::File::for_path(&backup))
            .unwrap();
        repo.update(|d| {
            d.sessions.clear();
            Ok(())
        })
        .unwrap();
        let previous = repo.import(&backup).unwrap();
        assert_eq!(repo.snapshot().sessions.len(), 1);
        assert!(previous.exists());
        fs::write(&backup, b"broken").unwrap();
        assert!(repo.import(&backup).is_err());
        assert_eq!(repo.snapshot().sessions.len(), 1);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn failed_atomic_write_keeps_previous_file_and_memory() {
        let root = folder();
        let repo = Repository::open(root.clone()).unwrap();
        let before = repo.snapshot().config.rutinas[0].nombre.clone();
        fs::remove_file(root.join("bosu_data.json")).unwrap();
        fs::create_dir(root.join("bosu_data.json")).unwrap();
        assert!(
            repo.update(|d| {
                d.config.rutinas[0].nombre = "changed".into();
                Ok(())
            })
            .is_err()
        );
        assert_eq!(repo.snapshot().config.rutinas[0].nombre, before);
        fs::remove_dir_all(root).unwrap();
    }
}
