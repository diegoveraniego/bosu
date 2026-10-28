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
            if let Err(error) = &result {
                label.set_text(&crate::i18n::tr(error));
                label.set_visible(true);
            } else {
                label.set_visible(false);
            }
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
fn title(row: &adw::ActionRow, doc: &RegistroSesion, index: usize) {
    let e = &doc.ejercicios[index];
    let detail = if e.estado == ExerciseState::Skipped {
        format!("{} · {}", e.planeado, crate::i18n::tr("Omitido"))
    } else if !e.realizado.trim().is_empty() {
        format!("{} · {}", e.planeado, e.realizado)
    } else {
        e.planeado.clone()
    };
    row.set_subtitle(&detail);
}
fn summary(label: &Label, doc: &RegistroSesion) {
    let done = doc
        .ejercicios
        .iter()
        .filter(|e| e.estado == ExerciseState::Done)
        .count();
    label.set_text(&if doc.estado == SessionState::Rest {
        String::new()
    } else {
        format!("{done}/{}", doc.ejercicios.len())
    });
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
            map_status.set_text(&format!(
                "{} · {count}",
                crate::i18n::tr(if before { "Antes" } else { "Después" })
            ));
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
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut doc = data
        .session_for_day(key, &today)
        .unwrap_or_else(|| RegistroSesion::new(routine));
    doc.prepare();
    let status = Label::builder()
        .wrap(true)
        .xalign(0.)
        .visible(false)
        .build();
    let state = Rc::new(DraftState {
        repo: repo.clone(),
        key: key.into(),
        doc: RefCell::new(doc),
        dirty: Cell::new(false),
        scheduled: RefCell::new(None),
        status: status.downgrade(),
    });
    *current.borrow_mut() = Some(state.clone());
    if data
        .sessions
        .iter()
        .any(|session| session.id == state.doc.borrow().id)
    {
        container.append(
            &Label::builder()
                .label(crate::i18n::tr(
                    "Tu sesión guardada de hoy. Puedes continuarla y guardar los cambios.",
                ))
                .wrap(true)
                .xalign(0.)
                .build(),
        );
    }
    container.append(&status);
    let retry = Button::with_label(&crate::i18n::tr("Reintentar guardar borrador"));
    retry.set_halign(gtk::Align::Start);
    retry.set_visible(false);
    status.connect_visible_notify(glib::clone!(
        #[weak]
        retry,
        move |label| retry.set_visible(label.is_visible())
    ));
    retry.connect_clicked(glib::clone!(
        #[strong]
        state,
        move |_| {
            state.dirty.set(true);
            let _ = state.flush();
        }
    ));
    container.append(&retry);
    let controls = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .halign(gtk::Align::Center)
        .build();
    for before in [true, false] {
        let map = if before {
            state.doc.borrow().mapa_antes.clone()
        } else {
            state.doc.borrow().mapa_despues.clone()
        };
        let label = crate::i18n::tr(if before { "Antes" } else { "Después" });
        let caption =
            Label::new(Some(&map.map_or(label.clone(), |m| {
                format!("{label} · {}", m.points.len())
            })));
        let content = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(6)
            .build();
        content.append(&gtk::Image::from_icon_name("avatar-default-symbolic"));
        content.append(&caption);
        let button = Button::builder()
            .child(&content)
            .css_classes(["flat"])
            .tooltip_text(crate::i18n::tr(if before {
                "Mapa antes de la sesión"
            } else {
                "Mapa después de la sesión"
            }))
            .build();
        button.connect_clicked(glib::clone!(
            #[strong]
            state,
            #[weak]
            caption,
            move |button| open_map(button, state.clone(), before, caption)
        ));
        controls.append(&button);
    }
    let options = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .build();
    let popover = gtk::Popover::builder().child(&options).build();
    controls.append(
        &gtk::MenuButton::builder()
            .icon_name("view-more-symbolic")
            .popover(&popover)
            .css_classes(["flat"])
            .tooltip_text(crate::i18n::tr("Opciones de la sesión"))
            .build(),
    );
    container.append(&controls);
    let progress = Label::builder()
        .xalign(1.)
        .css_classes(["dim-label"])
        .build();
    summary(&progress, &state.doc.borrow());
    if state.doc.borrow().estado == SessionState::Rest {
        container.append(
            &adw::StatusPage::builder()
                .icon_name("non-emergency-healthcare-symbolic")
                .title(crate::i18n::tr("Día de descanso"))
                .build(),
        );
    } else {
        let mut groups = std::collections::BTreeMap::<String, adw::PreferencesGroup>::new();
        let colors = ["group-blue", "group-green", "group-purple", "group-orange"];
        let exercises = state.doc.borrow().ejercicios.clone();
        for (index, exercise) in exercises.iter().enumerate() {
            let category = format!(
                "{}:{}",
                exercise.id.split(':').next().unwrap_or(""),
                exercise.categoria
            );
            if !groups.contains_key(&category) {
                let group = adw::PreferencesGroup::builder()
                    .title(&exercise.categoria)
                    .build();
                group.add_css_class(colors[groups.len() % colors.len()]);
                container.append(&group);
                groups.insert(category.clone(), group);
            }
            let done = gtk::CheckButton::builder()
                .active(exercise.estado == ExerciseState::Done)
                .valign(gtk::Align::Center)
                .tooltip_text(crate::i18n::tr("Marcar ejercicio como hecho"))
                .build();
            done.update_property(&[gtk::accessible::Property::Label(&exercise.nombre)]);
            let row = adw::ActionRow::builder()
                .use_markup(false)
                .title(&exercise.nombre)
                .title_lines(2)
                .subtitle_lines(2)
                .activatable_widget(&done)
                .build();
            row.add_prefix(&done);
            title(&row, &state.doc.borrow(), index);
            let details = adw::PreferencesGroup::new();
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
            let skipped = gtk::CheckButton::builder()
                .label(crate::i18n::tr("Omitido"))
                .active(exercise.estado == ExerciseState::Skipped)
                .build();
            details.add(&actual);
            details.add(&reason);
            details.add(&skipped);
            let box_details = Box::builder()
                .orientation(Orientation::Vertical)
                .margin_top(12)
                .margin_bottom(12)
                .margin_start(12)
                .margin_end(12)
                .width_request(290)
                .build();
            box_details.append(&details);
            let popover = gtk::Popover::builder().child(&box_details).build();
            row.add_suffix(
                &gtk::MenuButton::builder()
                    .icon_name("view-more-symbolic")
                    .popover(&popover)
                    .css_classes(["flat"])
                    .valign(gtk::Align::Center)
                    .tooltip_text(crate::i18n::tr("Detalles del ejercicio"))
                    .build(),
            );
            let guard = Rc::new(Cell::new(false));
            done.connect_toggled(glib::clone!(
                #[strong]
                state,
                #[strong]
                guard,
                #[weak]
                skipped,
                #[weak]
                row,
                #[weak]
                progress,
                move |check| {
                    if guard.get() {
                        return;
                    }
                    guard.set(true);
                    if check.is_active() {
                        skipped.set_active(false);
                    }
                    guard.set(false);
                    let mut doc = state.doc.borrow_mut();
                    doc.ejercicios[index].estado = if check.is_active() {
                        ExerciseState::Done
                    } else if skipped.is_active() {
                        ExerciseState::Skipped
                    } else {
                        ExerciseState::Pending
                    };
                    doc.prepare();
                    title(&row, &doc, index);
                    summary(&progress, &doc);
                    drop(doc);
                    state.changed();
                }
            ));
            skipped.connect_toggled(glib::clone!(
                #[strong]
                state,
                #[strong]
                guard,
                #[weak]
                done,
                #[weak]
                row,
                #[weak]
                progress,
                move |check| {
                    if guard.get() {
                        return;
                    }
                    guard.set(true);
                    if check.is_active() {
                        done.set_active(false);
                    }
                    guard.set(false);
                    let mut doc = state.doc.borrow_mut();
                    doc.ejercicios[index].estado = if check.is_active() {
                        ExerciseState::Skipped
                    } else if done.is_active() {
                        ExerciseState::Done
                    } else {
                        ExerciseState::Pending
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
                row,
                #[weak]
                progress,
                move |entry| {
                    let mut doc = state.doc.borrow_mut();
                    doc.ejercicios[index].realizado = entry.text().into();
                    doc.prepare();
                    title(&row, &doc, index);
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
            groups[&category].add(&row);
        }
    }
    container.append(&progress);
    let assessment = adw::PreferencesGroup::builder()
        .title(crate::i18n::tr("Evaluación"))
        .build();
    let pain_value = state.doc.borrow().dolor;
    let pain = gtk::CheckButton::builder()
        .label(crate::i18n::tr("Dolor"))
        .active(pain_value.is_some())
        .valign(gtk::Align::Center)
        .build();
    let scale = gtk::Scale::with_range(Orientation::Horizontal, 0., 10., 1.);
    scale.set_value(pain_value.unwrap_or(0.));
    scale.set_digits(0);
    scale.set_draw_value(true);
    scale.set_hexpand(true);
    scale.set_sensitive(pain_value.is_some());
    scale.update_property(&[gtk::accessible::Property::Label(&crate::i18n::tr(
        "Dolor · 0 a 10",
    ))]);
    let pain_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(8)
        .build();
    pain_box.append(&pain);
    pain_box.append(&scale);
    assessment.add(&pain_box);
    pain.connect_toggled(glib::clone!(
        #[strong]
        state,
        #[weak]
        scale,
        move |check| {
            scale.set_sensitive(check.is_active());
            state.doc.borrow_mut().dolor = check.is_active().then(|| scale.value());
            state.changed();
        }
    ));
    scale.connect_value_changed(glib::clone!(
        #[strong]
        state,
        #[weak]
        pain,
        move |scale| {
            if pain.is_active() {
                state.doc.borrow_mut().dolor = Some(scale.value());
                state.changed();
            }
        }
    ));
    let notes = adw::EntryRow::builder()
        .use_markup(false)
        .title(crate::i18n::tr("Notas de la sesión · opcional"))
        .text(&state.doc.borrow().notas)
        .build();
    assessment.add(&notes);
    notes.connect_changed(glib::clone!(
        #[strong]
        state,
        move |entry| {
            state.doc.borrow_mut().notas = entry.text().into();
            state.changed();
        }
    ));
    let extras = adw::ExpanderRow::builder()
        .title(crate::i18n::tr("Más detalles"))
        .build();
    let effort_value = state.doc.borrow().esfuerzo;
    let effort = adw::SwitchRow::builder()
        .title(crate::i18n::tr("Registrar esfuerzo"))
        .active(effort_value.is_some())
        .build();
    let spin = adw::SpinRow::with_range(0., 10., 1.);
    spin.set_title(&crate::i18n::tr("Esfuerzo · 0 a 10"));
    spin.set_value(effort_value.unwrap_or(0.));
    spin.set_sensitive(effort_value.is_some());
    extras.add_row(&effort);
    extras.add_row(&spin);
    effort.connect_active_notify(glib::clone!(
        #[strong]
        state,
        #[weak]
        spin,
        move |row| {
            spin.set_sensitive(row.is_active());
            state.doc.borrow_mut().esfuerzo = row.is_active().then(|| spin.value());
            state.changed();
        }
    ));
    spin.connect_value_notify(glib::clone!(
        #[strong]
        state,
        #[weak]
        effort,
        move |spin| {
            if effort.is_active() {
                state.doc.borrow_mut().esfuerzo = Some(spin.value());
                state.changed();
            }
        }
    ));
    let date = adw::EntryRow::builder()
        .use_markup(false)
        .title(crate::i18n::tr("Fecha · AAAA-MM-DD"))
        .text(&state.doc.borrow().fecha)
        .build();
    extras.add_row(&date);
    date.connect_changed(glib::clone!(
        #[strong]
        state,
        move |entry| {
            state.doc.borrow_mut().fecha = entry.text().into();
            state.changed();
        }
    ));
    assessment.add(&extras);
    container.append(&assessment);
    let save = Button::builder()
        .label(crate::i18n::tr(
            if state.doc.borrow().estado == SessionState::Rest {
                "Guardar descanso"
            } else {
                "Guardar sesión"
            },
        ))
        .css_classes(["suggested-action"])
        .margin_top(12)
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
                        SessionState::Partial => "Sesión parcial guardada.",
                        SessionState::Rest => "Descanso registrado.",
                        _ => "Sesión guardada.",
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
                        label.set_visible(true);
                    }
                }
            }
        }
    ));
    let discard = Button::builder()
        .label(crate::i18n::tr("Descartar borrador"))
        .css_classes(["flat"])
        .build();
    options.append(&discard);
    discard.connect_clicked(glib::clone!(#[strong] state,#[strong] current,#[strong] on_changed,#[weak] toasts,#[weak] container,move|button|{
        let Some(parent)=button.root().and_downcast::<gtk::Window>()else{return;};
        let dialog=adw::MessageDialog::new(Some(&parent),Some(&crate::i18n::tr("¿Descartar los cambios del borrador?")),Some(&crate::i18n::tr("Volverás a la sesión guardada de hoy si existe. Las sesiones guardadas y los demás borradores se conservan.")));
        dialog.add_responses(&[("cancel",&crate::i18n::tr("Cancelar")),("discard",&crate::i18n::tr("Descartar borrador"))]);dialog.set_default_response(Some("cancel"));dialog.set_close_response("cancel");dialog.set_response_appearance("discard",adw::ResponseAppearance::Destructive);
        dialog.connect_response(Some("discard"),glib::clone!(#[strong] state,#[strong] current,#[strong] on_changed,#[weak] toasts,#[weak] container,move|_,_|match state.repo.update(|data|{data.drafts.remove(&state.key);Ok(())}){
            Ok(())=>{state.cancel();render(&container,&state.repo,&state.key,current.clone(),on_changed.clone(),toasts.clone());},Err(error)=>toasts.add_toast(crate::toast(&error))
        }));dialog.present();
    }));
}
