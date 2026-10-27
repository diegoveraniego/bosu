use crate::{
    RegistroSesion,
    models::{ExerciseState, SessionState},
    storage::Repository,
};
use adw::prelude::*;
use gtk::{Box, Button, Label, Orientation, glib};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
pub struct Page {
    pub widget: Box,
    pub flush: Rc<dyn Fn() -> Result<(), String>>,
}
struct DraftState {
    repo: Repository,
    key: String,
    doc: RefCell<RegistroSesion>,
    dirty: Cell<bool>,
    scheduled: RefCell<Option<glib::SourceId>>,
    status: glib::WeakRef<Label>,
}
impl DraftState {
    fn flush(&self) -> Result<(), String> {
        if !self.dirty.get() {
            return Ok(());
        }
        let doc = self.doc.borrow().clone();
        let key = self.key.clone();
        let result = self.repo.update(|data| {
            data.drafts.insert(key.clone(), doc);
            data.selected_draft = Some(key);
            Ok(())
        });
        if let Some(label) = self.status.upgrade() {
            label.set_text(&crate::i18n::tr(match &result {
                Ok(_) => "Borrador guardado en este equipo.",
                Err(e) => e,
            }));
        }
        if result.is_ok() {
            self.dirty.set(false);
        }
        result
    }
    fn changed(self: &Rc<Self>) {
        self.dirty.set(true);
        if self.scheduled.borrow().is_none() {
            let weak = Rc::downgrade(self);
            let source =
                glib::timeout_add_local_once(std::time::Duration::from_millis(350), move || {
                    if let Some(state) = weak.upgrade() {
                        state.scheduled.borrow_mut().take();
                        let _ = state.flush();
                    }
                });
            *self.scheduled.borrow_mut() = Some(source);
        }
    }
    fn cancel(&self) {
        if let Some(source) = self.scheduled.borrow_mut().take() {
            source.remove();
        }
        self.dirty.set(false);
    }
}
impl Drop for DraftState {
    fn drop(&mut self) {
        if let Some(source) = self.scheduled.get_mut().take() {
            source.remove();
        }
    }
}
fn title(row: &adw::ExpanderRow, doc: &RegistroSesion, index: usize) {
    let e = &doc.ejercicios[index];
    row.set_subtitle(&crate::i18n::tr(&format!(
        "{} · {}",
        e.planeado,
        match e.estado {
            ExerciseState::Pending => "Pendiente",
            ExerciseState::Done => "Hecho",
            ExerciseState::Skipped => "Omitido",
        }
    )));
}
fn summary(label: &Label, doc: &RegistroSesion) {
    let done = doc
        .ejercicios
        .iter()
        .filter(|e| e.estado == ExerciseState::Done)
        .count();
    let skipped = doc
        .ejercicios
        .iter()
        .filter(|e| e.estado == ExerciseState::Skipped)
        .count();
    let text = if doc.estado == SessionState::Rest {
        "Puedes registrar descanso y cómo te sientes.".to_string()
    } else {
        format!(
            "{done}/{} ejercicios hechos · {skipped} omitidos. Puedes guardar una sesión parcial; hacer algo también es progreso.",
            doc.ejercicios.len()
        )
    };
    label.set_text(&crate::i18n::tr(&text));
}
fn open_map(button: &Button, state: Rc<DraftState>, before: bool, map_status: Label) {
    let Some(parent) = button.root().and_downcast::<gtk::Window>() else {
        return;
    };
    let window = adw::Window::builder()
        .transient_for(&parent)
        .modal(true)
        .default_width(430)
        .default_height(740)
        .title(crate::i18n::tr(if before {
            "Cómo estoy · antes"
        } else {
            "Cómo quedé · después"
        }))
        .build();
    let initial = if before {
        state.doc.borrow().mapa_antes.clone()
    } else {
        state.doc.borrow().mapa_despues.clone()
    };
    let session_id = state.doc.borrow().id.clone();
    let repository = state.repo.clone();
    let save: Rc<dyn Fn(crate::body_map::Snapshot) -> Result<(), String>> =
        Rc::new(move |snapshot| {
            let count = snapshot.points.len();
            if before {
                state.doc.borrow_mut().mapa_antes = Some(snapshot);
            } else {
                state.doc.borrow_mut().mapa_despues = Some(snapshot);
            }
            state.dirty.set(true);
            state.flush()?;
            map_status.set_text(&crate::i18n::tr(&format!(
                "{count} puntos en el mapa · borrador guardado"
            )));
            Ok(())
        });
    let panel = crate::body_map::build_linked(
        repository,
        initial,
        if before { "before" } else { "after" },
        session_id,
        save,
    );
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let done = Button::with_label(&crate::i18n::tr("Listo"));
    header.pack_end(&done);
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&panel.widget));
    window.set_content(Some(&toolbar));
    let flush = panel.flush;
    let skip_flush = Rc::new(Cell::new(false));
    done.connect_clicked(glib::clone!(
        #[weak]
        window,
        move |_| window.close()
    ));
    window.connect_close_request(glib::clone!(
        #[weak]
        window,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_| match if skip_flush.get() { Ok(()) } else { flush() } {
            Ok(()) => glib::Propagation::Proceed,
            Err(e) => {
                let dialog = adw::MessageDialog::new(
                    Some(&window),
                    Some(&crate::i18n::tr("No se pudo guardar el mapa")),
                    Some(&e),
                );
                dialog.add_response("close", &crate::i18n::tr("Seguir editando"));
                dialog.add_response("discard", &crate::i18n::tr("Cerrar sin guardar en disco"));
                dialog.set_default_response(Some("close"));
                dialog.set_close_response("close");
                dialog.connect_response(
                    Some("discard"),
                    glib::clone!(
                        #[weak]
                        window,
                        #[strong]
                        skip_flush,
                        move |_, _| {
                            skip_flush.set(true);
                            window.close();
                        }
                    ),
                );
                dialog.present();
                glib::Propagation::Stop
            }
        }
    ));
    window.present();
}

pub fn build(repo: Repository, on_changed: Rc<dyn Fn()>, toasts: adw::ToastOverlay) -> Page {
    let root = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(18)
        .margin_top(24)
        .margin_bottom(32)
        .margin_start(16)
        .margin_end(16)
        .build();
    root.append(
        &Label::builder()
            .label(crate::i18n::tr("Hoy, a tu ritmo"))
            .css_classes(["title-1"])
            .halign(gtk::Align::Start)
            .build(),
    );
    root.append(&Label::builder().label(crate::i18n::tr("Practica, adapta o descansa. Cada registro te ayuda a entender cómo responde tu cuerpo.")).wrap(true).xalign(0.).build());
    let data = repo.snapshot();
    let mut names = vec![crate::i18n::tr("Descanso o sensaciones")];
    names.extend(data.config.rutinas.iter().map(|r| r.nombre.clone()));
    let mut keys = vec!["__rest__".to_string()];
    keys.extend(data.config.rutinas.iter().map(|r| r.id.clone()));
    for (key, draft) in &data.drafts {
        if !keys.contains(key) {
            keys.push(key.clone());
            names.push(format!(
                "{} · {}",
                draft.rutina_nombre,
                crate::i18n::tr("Borrador recuperado")
            ));
        }
    }
    let selector =
        gtk::DropDown::from_strings(&names.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    selector.set_enable_search(true);
    selector.update_property(&[gtk::accessible::Property::Label(&crate::i18n::tr(
        "Rutina de hoy",
    ))]);
    let selected = data
        .selected_draft
        .or(data.config.ultima_rutina_seleccionada)
        .and_then(|id| keys.iter().position(|k| *k == id))
        .unwrap_or(0);
    selector.set_selected(selected as u32);
    root.append(&selector);
    let form = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(18)
        .build();
    root.append(&form);
    let current = Rc::new(RefCell::new(None::<Rc<DraftState>>));
    render(
        &form,
        &repo,
        &keys[selected],
        current.clone(),
        on_changed.clone(),
        toasts.clone(),
    );
    let guard = Rc::new(Cell::new(false));
    let previous = Rc::new(Cell::new(selected as u32));
    selector.connect_selected_notify(glib::clone!(
        #[weak]
        form,
        #[strong]
        current,
        #[strong]
        repo,
        #[strong]
        keys,
        #[strong]
        on_changed,
        #[weak]
        toasts,
        move |dd| {
            if guard.get() {
                return;
            }
            if let Some(state) = current.borrow().as_ref()
                && let Err(e) = state.flush()
            {
                toasts.add_toast(crate::toast(&crate::i18n::tr(&format!(
                    "El borrador sigue abierto: {e}"
                ))));
                guard.set(true);
                dd.set_selected(previous.get());
                guard.set(false);
                return;
            }
            if let Some(key) = keys.get(dd.selected() as usize) {
                previous.set(dd.selected());
                render(
                    &form,
                    &repo,
                    key,
                    current.clone(),
                    on_changed.clone(),
                    toasts.clone(),
                );
            }
        }
    ));
    let flush: Rc<dyn Fn() -> Result<(), String>> = Rc::new(move || {
        current
            .borrow()
            .as_ref()
            .map_or(Ok(()), |state| state.flush())
    });
    Page {
        widget: root,
        flush,
    }
}
fn render(
    container: &Box,
    repo: &Repository,
    key: &str,
    current: Rc<RefCell<Option<Rc<DraftState>>>>,
    on_changed: Rc<dyn Fn()>,
    toasts: adw::ToastOverlay,
) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
    let data = repo.snapshot();
    let routine = data.config.rutinas.iter().find(|r| r.id == key);
    let mut doc = data
        .drafts
        .get(key)
        .cloned()
        .unwrap_or_else(|| RegistroSesion::new(routine));
    doc.prepare();
    let status = Label::builder()
        .wrap(true)
        .xalign(0.)
        .css_classes(["dim-label"])
        .build();
    status.set_text(&crate::i18n::tr(if data.drafts.contains_key(key) {
        "Borrador recuperado. Puedes continuar donde lo dejaste."
    } else {
        "Los cambios se guardan como borrador en este equipo."
    }));
    let state = Rc::new(DraftState {
        repo: repo.clone(),
        key: key.into(),
        doc: RefCell::new(doc),
        dirty: Cell::new(false),
        scheduled: RefCell::new(None),
        status: status.downgrade(),
    });
    *current.borrow_mut() = Some(state.clone());
    container.append(&status);
    let retry = Button::with_label(&crate::i18n::tr("Reintentar guardar borrador"));
    retry.set_halign(gtk::Align::Start);
    retry.connect_clicked(glib::clone!(
        #[strong]
        state,
        move |_| {
            state.dirty.set(true);
            let _ = state.flush();
        }
    ));
    container.append(&retry);
    let before = adw::PreferencesGroup::builder()
        .title(crate::i18n::tr("1 · Cómo estoy"))
        .description(crate::i18n::tr(
            "El mapa es opcional y queda vinculado a esta sesión.",
        ))
        .build();
    let before_status = Label::builder()
        .label(crate::i18n::tr(&format!(
            "{} puntos",
            state
                .doc
                .borrow()
                .mapa_antes
                .as_ref()
                .map_or(0, |m| m.points.len())
        )))
        .wrap(true)
        .build();
    let before_row = adw::ActionRow::builder()
        .use_markup(false)
        .title(crate::i18n::tr("Mapa antes de la sesión"))
        .build();
    let before_button = Button::builder()
        .label(crate::i18n::tr("Registrar"))
        .valign(gtk::Align::Center)
        .build();
    before_row.add_suffix(&before_button);
    before.add(&before_row);
    container.append(&before);
    container.append(&before_status);
    before_button.connect_clicked(glib::clone!(
        #[strong]
        state,
        #[weak]
        before_status,
        move |button| open_map(button, state.clone(), true, before_status)
    ));
    let progress = Label::builder().wrap(true).xalign(0.).build();
    summary(&progress, &state.doc.borrow());
    if state.doc.borrow().estado == SessionState::Rest {
        container.append(
            &Label::builder()
                .label(crate::i18n::tr(
                    "Descansar también forma parte de tu proceso.",
                ))
                .wrap(true)
                .css_classes(["title-3"])
                .build(),
        );
    } else {
        container.append(
            &Label::builder()
                .label(crate::i18n::tr("2 · Lo que hice"))
                .css_classes(["title-2"])
                .xalign(0.)
                .build(),
        );
        let mut groups = std::collections::BTreeMap::<String, adw::PreferencesGroup>::new();
        let mut order = Vec::new();
        let colors = ["group-blue", "group-green", "group-purple", "group-orange"];
        let exercises = state.doc.borrow().ejercicios.clone();
        for (index, exercise) in exercises.iter().enumerate() {
            let category_key = format!(
                "{}:{}",
                exercise.id.split(':').next().unwrap_or(""),
                exercise.categoria
            );
            if !groups.contains_key(&category_key) {
                let group = adw::PreferencesGroup::builder()
                    .title(&exercise.categoria)
                    .build();
                group.add_css_class(colors[order.len() % colors.len()]);
                container.append(&group);
                groups.insert(category_key.clone(), group);
                order.push(exercise.categoria.clone());
            }
            let row = adw::ExpanderRow::builder()
                .use_markup(false)
                .title(&exercise.nombre)
                .title_lines(2)
                .subtitle_lines(2)
                .build();
            title(&row, &state.doc.borrow(), index);
            let choice = crate::i18n::dropdown(&["Pendiente", "Hecho", "Omitido"]);
            choice.set_valign(gtk::Align::Center);
            choice.set_selected(match exercise.estado {
                ExerciseState::Pending => 0,
                ExerciseState::Done => 1,
                ExerciseState::Skipped => 2,
            });
            choice.update_property(&[gtk::accessible::Property::Label(&format!(
                "Estado de {}",
                exercise.nombre
            ))]);
            row.add_suffix(&choice);
            let actual = adw::EntryRow::builder()
                .use_markup(false)
                .title(crate::i18n::tr(
                    "Lo que hice · repeticiones, peso o asistencia",
                ))
                .text(&exercise.realizado)
                .build();
            let reason = adw::EntryRow::builder()
                .use_markup(false)
                .title(crate::i18n::tr("Nota o motivo de omisión · opcional"))
                .text(&exercise.motivo)
                .build();
            row.add_row(&actual);
            row.add_row(&reason);
            choice.connect_selected_notify(glib::clone!(
                #[strong]
                state,
                #[weak]
                row,
                #[weak]
                progress,
                move |dd| {
                    let mut doc = state.doc.borrow_mut();
                    doc.ejercicios[index].estado = match dd.selected() {
                        1 => ExerciseState::Done,
                        2 => ExerciseState::Skipped,
                        _ => ExerciseState::Pending,
                    };
                    doc.prepare();
                    title(&row, &doc, index);
                    summary(&progress, &doc);
                    drop(doc);
                    state.changed();
                }
            ));
            actual.connect_changed(glib::clone!(
                #[strong]
                state,
                #[weak]
                progress,
                move |entry| {
                    let mut doc = state.doc.borrow_mut();
                    doc.ejercicios[index].realizado = entry.text().into();
                    doc.prepare();
                    summary(&progress, &doc);
                    drop(doc);
                    state.changed();
                }
            ));
            reason.connect_changed(glib::clone!(
                #[strong]
                state,
                move |entry| {
                    state.doc.borrow_mut().ejercicios[index].motivo = entry.text().into();
                    state.changed();
                }
            ));
            groups[&category_key].add(&row);
        }
    }
    container.append(&progress);
    let after = adw::PreferencesGroup::builder()
        .title(crate::i18n::tr("3 · Cómo quedé"))
        .description(crate::i18n::tr(
            "Dolor y esfuerzo son registros distintos. Puedes dejar cualquiera sin medir.",
        ))
        .build();
    let after_row = adw::ActionRow::builder()
        .use_markup(false)
        .title(crate::i18n::tr("Mapa después de la sesión"))
        .build();
    let after_button = Button::builder()
        .label(crate::i18n::tr("Registrar"))
        .valign(gtk::Align::Center)
        .build();
    after_row.add_suffix(&after_button);
    after.add(&after_row);
    let after_status = Label::builder()
        .label(crate::i18n::tr(&format!(
            "{} puntos",
            state
                .doc
                .borrow()
                .mapa_despues
                .as_ref()
                .map_or(0, |m| m.points.len())
        )))
        .wrap(true)
        .build();
    after_button.connect_clicked(glib::clone!(
        #[strong]
        state,
        #[weak]
        after_status,
        move |button| open_map(button, state.clone(), false, after_status)
    ));
    for pain in [true, false] {
        let value = if pain {
            state.doc.borrow().dolor
        } else {
            state.doc.borrow().esfuerzo
        };
        let record = adw::SwitchRow::builder()
            .title(crate::i18n::tr(if pain {
                "Registrar dolor"
            } else {
                "Registrar esfuerzo"
            }))
            .active(value.is_some())
            .build();
        let spin = adw::SpinRow::with_range(0., 10., 1.);
        spin.set_title(&crate::i18n::tr(if pain {
            "Dolor · 0 a 10"
        } else {
            "Esfuerzo · 0 a 10"
        }));
        spin.set_value(value.unwrap_or(0.));
        spin.set_sensitive(value.is_some());
        after.add(&record);
        after.add(&spin);
        record.connect_active_notify(glib::clone!(
            #[strong]
            state,
            #[weak]
            spin,
            move |row| {
                spin.set_sensitive(row.is_active());
                let value = row.is_active().then(|| spin.value());
                if pain {
                    state.doc.borrow_mut().dolor = value;
                } else {
                    state.doc.borrow_mut().esfuerzo = value;
                }
                state.changed();
            }
        ));
        spin.connect_value_notify(glib::clone!(
            #[strong]
            state,
            #[weak]
            record,
            move |row| {
                if record.is_active() {
                    if pain {
                        state.doc.borrow_mut().dolor = Some(row.value());
                    } else {
                        state.doc.borrow_mut().esfuerzo = Some(row.value());
                    }
                    state.changed();
                }
            }
        ));
    }
    let notes = adw::EntryRow::builder()
        .use_markup(false)
        .title(crate::i18n::tr("Notas de la sesión · opcional"))
        .text(&state.doc.borrow().notas)
        .build();
    after.add(&notes);
    notes.connect_changed(glib::clone!(
        #[strong]
        state,
        move |entry| {
            state.doc.borrow_mut().notas = entry.text().into();
            state.changed();
        }
    ));
    let date = adw::EntryRow::builder()
        .use_markup(false)
        .title(crate::i18n::tr("Fecha · AAAA-MM-DD"))
        .text(&state.doc.borrow().fecha)
        .build();
    after.add(&date);
    date.connect_changed(glib::clone!(
        #[strong]
        state,
        move |entry| {
            state.doc.borrow_mut().fecha = entry.text().into();
            state.changed();
        }
    ));
    container.append(&after);
    container.append(&after_status);
    let save = Button::builder()
        .label(crate::i18n::tr(
            if state.doc.borrow().estado == SessionState::Rest {
                "Guardar descanso"
            } else {
                "Guardar mi avance"
            },
        ))
        .css_classes(["suggested-action", "pill"])
        .build();
    container.append(&save);
    save.connect_clicked(glib::clone!(
        #[strong]
        state,
        #[strong]
        current,
        #[strong]
        on_changed,
        #[weak]
        toasts,
        #[weak]
        container,
        move |button| {
            button.set_sensitive(false);
            let mut session = state.doc.borrow().clone();
            session.prepare();
            session.registrado_en = chrono::Local::now().to_rfc3339();
            match state.repo.save_session(&state.key, session.clone()) {
                Ok(()) => {
                    state.cancel();
                    toasts.add_toast(crate::toast(&crate::i18n::tr(match session.estado {
                        SessionState::Partial => {
                            "Sesión parcial guardada. Hacer una parte también cuenta."
                        }
                        SessionState::Rest => "Descanso registrado.",
                        SessionState::Complete => "Sesión guardada.",
                        _ => "Tus sensaciones quedaron registradas.",
                    })));
                    on_changed();
                    render(
                        &container,
                        &state.repo,
                        &state.key,
                        current.clone(),
                        on_changed.clone(),
                        toasts.clone(),
                    );
                }
                Err(error) => {
                    button.set_sensitive(true);
                    if let Some(label) = state.status.upgrade() {
                        label.set_text(&crate::i18n::tr(&format!(
                            "No se pudo guardar. Tu registro sigue aquí: {error}"
                        )));
                    }
                }
            }
        }
    ));
    let discard = Button::with_label(&crate::i18n::tr("Descartar borrador y empezar de nuevo"));
    container.append(&discard);
    discard.connect_clicked(glib::clone!(#[strong] state,#[strong] current,#[strong] on_changed,#[weak] toasts,#[weak] container,move|button|{
        let Some(parent)=button.root().and_downcast::<gtk::Window>()else{return;};
        let dialog=adw::MessageDialog::new(Some(&parent),Some(&crate::i18n::tr("¿Empezar un nuevo borrador?")),Some(&crate::i18n::tr("Solo se descartará el borrador de esta rutina. Las sesiones guardadas y los demás borradores se conservan.")));
        dialog.add_responses(&[("cancel",&crate::i18n::tr("Cancelar")),("discard",&crate::i18n::tr("Descartar borrador"))]);dialog.set_default_response(Some("cancel"));dialog.set_close_response("cancel");dialog.set_response_appearance("discard",adw::ResponseAppearance::Destructive);
        dialog.connect_response(Some("discard"),glib::clone!(#[strong] state,#[strong] current,#[strong] on_changed,#[weak] toasts,#[weak] container,move|_,_|{
            match state.repo.update(|data|{data.drafts.remove(&state.key);Ok(())}) {
                Ok(())=>{state.cancel();render(&container,&state.repo,&state.key,current.clone(),on_changed.clone(),toasts.clone());},
                Err(error)=>toasts.add_toast(crate::toast(&crate::i18n::tr(&format!("No se pudo descartar el borrador: {error}")))),
            }
        }));dialog.present();
    }));
}
