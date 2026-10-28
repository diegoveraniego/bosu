//! Guided session experiment. Edits are durable drafts until explicitly saved.
use crate::{RegistroSesion, models::ExerciseState, storage::Repository};
use adw::prelude::*;
use gtk::{Box, Button, Label, Orientation, glib};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

struct Exercise {
    name: String,
    plan: String,
    category: String,
    color: usize,
    done: bool,
    skipped: bool,
}
#[derive(Default)]
struct Preview {
    exercises: Vec<Exercise>,
    doc: Option<RegistroSesion>,
    key: String,
    index: usize,
    timer_seconds: u64,
    elapsed: Duration,
    started: Option<Instant>,
}
impl Preview {
    fn document(&self) -> Option<RegistroSesion> {
        let mut doc = self.doc.clone()?;
        for (record, exercise) in doc.ejercicios.iter_mut().zip(&self.exercises) {
            record.estado = if exercise.done {
                ExerciseState::Done
            } else if exercise.skipped {
                ExerciseState::Skipped
            } else {
                ExerciseState::Pending
            };
        }
        doc.prepare();
        Some(doc)
    }
    fn persist(&self, repo: &Repository) -> Result<(), String> {
        let Some(doc) = self.document() else {
            return Ok(());
        };
        repo.update(|data| {
            data.drafts.insert(self.key.clone(), doc);
            data.selected_draft = Some(self.key.clone());
            Ok(())
        })
    }
    fn reset_timer(&mut self) {
        self.elapsed = Duration::ZERO;
        self.started = None;
    }
    fn elapsed(&self) -> Duration {
        self.elapsed + self.started.map_or(Duration::ZERO, |start| start.elapsed())
    }
    fn toggle_timer(&mut self) {
        if self.timer_seconds == 0 || self.index >= self.exercises.len() {
            return;
        }
        if let Some(started) = self.started.take() {
            self.elapsed += started.elapsed();
        } else {
            if self.elapsed.as_secs() >= self.timer_seconds {
                self.elapsed = Duration::ZERO;
            }
            self.started = Some(Instant::now());
        }
    }
    fn move_by(&mut self, offset: isize) {
        self.index = self
            .index
            .saturating_add_signed(offset)
            .min(self.exercises.len());
        self.reset_timer();
    }
    fn complete(&mut self) {
        if let Some(exercise) = self.exercises.get_mut(self.index) {
            exercise.done = true;
            exercise.skipped = false;
            self.move_by(1);
        }
    }
    fn skip(&mut self) {
        if let Some(exercise) = self.exercises.get_mut(self.index) {
            exercise.done = false;
            exercise.skipped = true;
            self.move_by(1);
        }
    }
}
const COLORS: [&str; 4] = ["focus-blue", "focus-green", "focus-purple", "focus-orange"];

pub fn present(parent: &adw::ApplicationWindow, repo: &Repository) -> adw::Window {
    let window = adw::Window::builder()
        .decorated(false)
        .title("Bosu · Modo enfoque (prueba)")
        .default_width(560)
        .default_height(760)
        .build();
    window.set_application(parent.application().as_ref());
    window.set_widget_name("focus-session");
    let root = Box::new(Orientation::Vertical, 0);
    root.add_css_class("focus-preview");
    root.add_css_class(COLORS[0]);
    let handle = gtk::WindowHandle::new();
    handle.set_child(Some(&root));
    window.set_content(Some(&handle));
    let styles = adw::StyleManager::default();
    let update_contrast: Rc<dyn Fn()> = Rc::new(glib::clone!(
        #[weak]
        root,
        #[weak]
        styles,
        move || {
            if styles.is_high_contrast() {
                root.add_css_class("focus-high-contrast");
            } else {
                root.remove_css_class("focus-high-contrast");
            }
        }
    ));
    update_contrast();
    styles.connect_high_contrast_notify(glib::clone!(
        #[strong]
        update_contrast,
        move |_| update_contrast()
    ));
    let controls = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .halign(gtk::Align::End)
        .margin_start(16)
        .margin_end(16)
        .margin_bottom(12)
        .build();
    let close = Button::builder()
        .icon_name("window-close-symbolic")
        .tooltip_text("Salir del modo enfoque y conservar el borrador")
        .css_classes(["flat"])
        .build();
    close.connect_clicked(glib::clone!(
        #[weak]
        window,
        move |_| window.close()
    ));
    let fullscreen = Button::builder()
        .icon_name("view-fullscreen-symbolic")
        .tooltip_text("Alternar pantalla completa")
        .build();
    fullscreen.connect_clicked(glib::clone!(
        #[weak]
        window,
        move |_| {
            if window.is_fullscreen() {
                window.unfullscreen();
            } else {
                window.fullscreen();
            }
        }
    ));
    fullscreen.add_css_class("flat");

    let appearance = gtk::DropDown::from_strings(&["Sistema", "Claro", "Oscuro"]);
    appearance.update_property(&[gtk::accessible::Property::Label("Apariencia de Bosu")]);
    appearance.set_selected(crate::appearance::index(
        &repo.snapshot().preferences.appearance,
    ));
    let stack = gtk::Stack::builder()
        .vexpand(true)
        .transition_type(gtk::StackTransitionType::Crossfade)
        .build();
    root.append(&stack);
    let setup = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(24)
        .margin_start(32)
        .margin_end(32)
        .margin_top(40)
        .margin_bottom(40)
        .valign(gtk::Align::Center)
        .build();
    let heading = Label::builder()
        .label("Una cosa a la vez.")
        .wrap(true)
        .css_classes(["focus-heading"])
        .build();
    setup.append(&heading);
    setup.append(
        &Label::builder()
            .label("Tu rutina de hoy, a tu ritmo.")
            .wrap(true)
            .justify(gtk::Justification::Center)
            .build(),
    );
    let data = repo.snapshot();
    let routines = data.config.rutinas.clone();
    let selector = gtk::DropDown::from_strings(
        &routines
            .iter()
            .map(|r| r.nombre.as_str())
            .collect::<Vec<_>>(),
    );
    selector.set_enable_search(true);
    selector.update_property(&[gtk::accessible::Property::Label("Rutina para probar")]);
    let initial = data
        .selected_draft
        .as_ref()
        .or(data.config.ultima_rutina_seleccionada.as_ref())
        .and_then(|key| routines.iter().position(|r| &r.id == key))
        .unwrap_or(0);
    selector.set_selected(initial as u32);
    setup.append(&selector);
    let setup_progress = Label::builder()
        .wrap(true)
        .justify(gtk::Justification::Center)
        .build();
    setup.append(&setup_progress);
    let begin = Button::with_label("Continuar");
    begin.add_css_class("focus-primary");
    begin.add_css_class("suggested-action");
    begin.add_css_class("pill");
    begin.set_sensitive(!routines.is_empty());
    setup.append(&begin);
    stack.add_named(&setup, Some("setup"));

    let card = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .margin_start(28)
        .margin_end(28)
        .margin_top(8)
        .margin_bottom(24)
        .build();
    let progress = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(3)
        .homogeneous(true)
        .margin_top(12)
        .margin_bottom(4)
        .build();
    progress.set_widget_name("focus-progress");
    card.append(&progress);
    let middle = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .vexpand(true)
        .valign(gtk::Align::Fill)
        .build();
    let category = Label::builder()
        .wrap(true)
        .justify(gtk::Justification::Center)
        .css_classes(["focus-category"])
        .build();
    let exercise = crate::focus_title::FocusTitle::new();
    exercise.set_focusable(true);
    exercise.set_vexpand(true);
    exercise.set_hexpand(true);
    exercise.set_widget_name("focus-exercise-title");
    let plan = Label::builder()
        .wrap(true)
        .justify(gtk::Justification::Center)
        .css_classes(["focus-plan"])
        .build();
    middle.append(&exercise);
    let clock = Label::builder().css_classes(["focus-clock"]).build();
    middle.append(&clock);
    let timer_message = Label::builder()
        .wrap(true)
        .justify(gtk::Justification::Center)
        .build();
    middle.append(&timer_message);
    let timer_options = gtk::DropDown::from_strings(&[
        "Sin temporizador",
        "30 segundos",
        "1 minuto",
        "90 segundos",
        "2 minutos",
    ]);
    timer_options.set_halign(gtk::Align::Center);
    timer_options.update_property(&[gtk::accessible::Property::Label(
        "Duración del temporizador opcional",
    )]);

    let timer_button = Button::with_label("Iniciar temporizador · Espacio");
    timer_button.set_halign(gtk::Align::Center);

    card.append(&middle);
    let actions = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(20)
        .halign(gtk::Align::Center)
        .build();
    let done = Button::builder()
        .icon_name("emblem-ok-symbolic")
        .tooltip_text("Marcar como hecho y continuar (Enter)")
        .build();
    done.set_widget_name("focus-done");
    done.add_css_class("suggested-action");
    done.add_css_class("circular");
    done.set_size_request(56, 56);
    done.update_property(&[gtk::accessible::Property::Label("Listo")]);
    let skip = Button::builder()
        .icon_name("media-skip-forward-symbolic")
        .tooltip_text("Omitir este ejercicio y continuar (S)")
        .build();
    skip.set_widget_name("focus-skip");
    skip.add_css_class("circular");
    skip.set_size_request(56, 56);
    skip.update_property(&[gtk::accessible::Property::Label("Omitir")]);
    actions.append(&done);
    actions.append(&skip);

    let navigation = Box::new(Orientation::Horizontal, 12);
    navigation.set_homogeneous(true);
    let back = Button::with_label("← Anterior");
    let next = Button::with_label("Siguiente →");
    navigation.append(&back);
    navigation.append(&next);

    let finish = Button::with_label("Terminar por hoy");
    let choose = Button::with_label("Cambiar rutina");
    choose.add_css_class("flat");

    let options = gtk::MenuButton::builder()
        .icon_name("view-more-symbolic")
        .tooltip_text("Opciones de la sesión")
        .build();
    let options_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .build();
    options_box.append(&actions);
    options_box.append(&Label::new(Some("Enter · listo    S · omitir")));
    options_box.append(&Label::new(Some("Temporizador opcional")));
    options_box.append(&timer_options);
    options_box.append(&timer_button);
    options_box.append(&navigation);
    options_box.append(&finish);
    options_box.append(&choose);
    options_box.append(&Label::new(Some("Apariencia")));
    options_box.append(&appearance);
    options_box.append(&fullscreen);
    options_box.append(&close);
    let popover = gtk::Popover::new();
    let options_scroll = gtk::ScrolledWindow::builder()
        .child(&options_box)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .max_content_height(480)
        .build();
    popover.set_child(Some(&options_scroll));
    fullscreen.connect_clicked(glib::clone!(
        #[weak]
        popover,
        move |_| popover.popdown()
    ));
    options.set_popover(Some(&popover));
    options.add_css_class("flat");
    let info = gtk::MenuButton::builder()
        .icon_name("help-about-symbolic")
        .tooltip_text("Descripción del ejercicio")
        .css_classes(["flat"])
        .build();
    info.update_property(&[gtk::accessible::Property::Label(
        "Descripción del ejercicio",
    )]);
    let details = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(20)
        .margin_end(20)
        .width_request(240)
        .build();
    let description = Label::builder()
        .wrap(true)
        .max_width_chars(40)
        .xalign(0.)
        .css_classes(["title-3"])
        .build();
    details.append(&description);
    details.append(&category);
    details.append(&plan);
    let info_popover = gtk::Popover::new();
    info_popover.set_child(Some(&details));
    info.set_popover(Some(&info_popover));
    controls.append(&info);
    controls.append(&options);

    stack.add_named(&card, Some("card"));
    let result = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(24)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(32)
        .margin_end(32)
        .build();
    let result_title = Label::builder()
        .label("Por hoy, está bien.")
        .wrap(true)
        .css_classes(["focus-heading"])
        .build();
    result.append(&result_title);
    let result_count = Label::builder().wrap(true).build();
    result.append(&result_count);
    let review = Button::with_label("Seguir practicando");
    result.append(&review);
    let assessment = gtk::Expander::builder()
        .label("Cómo quedé · opcional")
        .build();
    let assessment_box = Box::new(Orientation::Vertical, 12);
    let notes = adw::EntryRow::builder().title("Notas de la sesión").build();
    assessment_box.append(&notes);
    let pain = gtk::DropDown::from_strings(&[
        "Dolor · sin registrar",
        "Dolor · 0",
        "Dolor · 1",
        "Dolor · 2",
        "Dolor · 3",
        "Dolor · 4",
        "Dolor · 5",
        "Dolor · 6",
        "Dolor · 7",
        "Dolor · 8",
        "Dolor · 9",
        "Dolor · 10",
    ]);
    let effort = gtk::DropDown::from_strings(&[
        "Esfuerzo · sin registrar",
        "Esfuerzo · 0",
        "Esfuerzo · 1",
        "Esfuerzo · 2",
        "Esfuerzo · 3",
        "Esfuerzo · 4",
        "Esfuerzo · 5",
        "Esfuerzo · 6",
        "Esfuerzo · 7",
        "Esfuerzo · 8",
        "Esfuerzo · 9",
        "Esfuerzo · 10",
    ]);
    assessment_box.append(&pain);
    assessment_box.append(&effort);
    assessment.set_child(Some(&assessment_box));
    result.append(&assessment);
    let save = Button::with_label("Guardar avance");
    save.add_css_class("focus-primary");
    save.add_css_class("suggested-action");
    save.add_css_class("pill");
    result.append(&save);
    let result_scroll = gtk::ScrolledWindow::builder()
        .child(&result)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    stack.add_named(&result_scroll, Some("result"));
    stack.set_visible_child_name("setup");
    root.append(&controls);
    let state = Rc::new(RefCell::new(Preview::default()));
    let initializing = Rc::new(Cell::new(false));
    let status = Label::builder()
        .wrap(true)
        .margin_start(24)
        .margin_end(24)
        .visible(false)
        .build();
    root.append(&status);
    let appearance_guard = Rc::new(Cell::new(false));
    appearance.connect_selected_notify(glib::clone!(
        #[strong]
        repo,
        #[weak]
        status,
        #[strong]
        appearance_guard,
        move |dd| {
            if appearance_guard.get() {
                return;
            }
            let value = crate::appearance::value(dd.selected());
            let previous = repo.snapshot().preferences.appearance;
            match repo.update(|data| {
                data.preferences.appearance = value.into();
                Ok(())
            }) {
                Ok(()) => crate::appearance::apply(value),
                Err(error) => {
                    appearance_guard.set(true);
                    dd.set_selected(crate::appearance::index(&previous));
                    appearance_guard.set(false);
                    status.set_text(&format!("No se pudo guardar la apariencia: {error}"));
                    status.set_visible(true);
                }
            }
        }
    ));
    let persist: Rc<dyn Fn() -> bool> = Rc::new(glib::clone!(
        #[strong]
        repo,
        #[strong]
        state,
        #[weak]
        status,
        #[upgrade_or]
        false,
        move || {
            match state.borrow().persist(&repo) {
                Ok(()) => {
                    status.set_visible(false);
                    true
                }
                Err(error) => {
                    status.set_text(&format!("No se pudo guardar el borrador: {error}"));
                    status.set_visible(true);
                    false
                }
            }
        }
    ));
    let setup_refresh: Rc<dyn Fn()> = Rc::new(glib::clone!(
        #[strong]
        repo,
        #[strong]
        routines,
        #[weak]
        selector,
        #[weak]
        setup_progress,
        #[weak]
        begin,
        move || {
            let Some(routine) = routines.get(selector.selected() as usize) else {
                return;
            };
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            let doc = repo.snapshot().session_for_day(&routine.id, &today);
            let completed = doc.as_ref().map_or(0, |doc| {
                doc.ejercicios
                    .iter()
                    .filter(|e| e.estado == ExerciseState::Done)
                    .count()
            });
            setup_progress.set_text(&format!("{completed} ejercicios hechos hoy"));
            begin.set_label(if doc.is_some() {
                "Continuar"
            } else {
                "Comenzar"
            });
        }
    ));
    setup_refresh();
    selector.connect_selected_notify(glib::clone!(
        #[strong]
        setup_refresh,
        move |_| setup_refresh()
    ));
    let progress_signature = Rc::new(RefCell::new(Vec::<(bool, bool, bool)>::new()));
    let refresh: Rc<dyn Fn()> = Rc::new(glib::clone!(
        #[strong]
        state,
        #[weak]
        stack,
        #[weak]
        skip,
        #[weak]
        result_count,
        #[weak]
        root,
        #[strong]
        progress_signature,
        #[weak]
        progress,
        #[weak]
        description,
        #[weak]
        info,
        #[weak]
        category,
        #[weak]
        exercise,
        #[weak]
        plan,
        #[weak]
        clock,
        #[weak]
        timer_button,
        #[weak]
        timer_message,
        #[weak]
        done,
        #[weak]
        back,
        #[weak]
        next,
        #[weak]
        timer_options,
        move || {
            let mut preview = state.borrow_mut();
            let remaining = preview
                .timer_seconds
                .saturating_sub(preview.elapsed().as_secs());
            if preview.timer_seconds > 0 && remaining == 0 && preview.started.is_some() {
                preview.elapsed = Duration::from_secs(preview.timer_seconds);
                preview.started = None;
            }
            let completed = preview.exercises.iter().filter(|e| e.done).count();
            let skipped = preview.exercises.iter().filter(|e| e.skipped).count();
            let item = preview.exercises.get(preview.index);
            for color in COLORS {
                root.remove_css_class(color);
            }
            root.add_css_class(COLORS[item.map_or(0, |e| e.color % COLORS.len())]);
            let signature: Vec<_> = preview
                .exercises
                .iter()
                .enumerate()
                .map(|(i, e)| (e.done, e.skipped, i == preview.index))
                .collect();
            if *progress_signature.borrow() != signature {
                while let Some(child) = progress.first_child() {
                    progress.remove(&child);
                }
                for (i, &(done, skipped, current)) in signature.iter().enumerate() {
                    let segment = Box::new(Orientation::Horizontal, 0);
                    segment.add_css_class("focus-step");
                    segment.set_hexpand(true);
                    if done {
                        segment.add_css_class("done");
                    } else if skipped {
                        segment.add_css_class("skipped");
                    }
                    if current {
                        segment.add_css_class("current");
                    }
                    let label = format!(
                        "{} · {}{}",
                        preview.exercises[i].name,
                        if done {
                            "Hecho"
                        } else if skipped {
                            "Omitido"
                        } else {
                            "Pendiente"
                        },
                        if current { " · actual" } else { "" }
                    );
                    segment.set_tooltip_text(Some(&label));
                    segment.update_property(&[gtk::accessible::Property::Label(&label)]);
                    progress.append(&segment);
                }
                *progress_signature.borrow_mut() = signature;
            }
            let progress_label = format!(
                "{completed} hechos, {skipped} omitidos, {} pendientes",
                preview.exercises.len() - completed - skipped
            );
            progress.update_property(&[gtk::accessible::Property::Label(&progress_label)]);
            progress.set_tooltip_text(Some(&progress_label));
            if let Some(item) = item {
                category.set_text(&item.category);
                exercise.set_text(&item.name);
                description.set_text(&item.name);
                plan.set_text(&item.plan);
                let action_label = if item.done {
                    "Continuar"
                } else if item.skipped {
                    "Hacer este ejercicio"
                } else {
                    "Listo"
                };
                done.update_property(&[gtk::accessible::Property::Label(action_label)]);
                done.set_tooltip_text(Some(&format!("{action_label} (Enter)")));
            } else {
                category.set_text("Por ahora, listo.");
                exercise.set_text("Hacer una parte también cuenta.");
                plan.set_text(&format!(
                    "{completed} de {} ejercicios marcados en esta vista.",
                    preview.exercises.len()
                ));
                done.update_property(&[gtk::accessible::Property::Label("Ver resumen")]);
                if stack.visible_child_name().as_deref() == Some("card") {
                    stack.set_visible_child_name("result");
                }
            }
            let has_item = item.is_some();
            done.set_sensitive(has_item);
            skip.set_sensitive(has_item);
            back.set_sensitive(preview.index > 0);
            next.set_sensitive(has_item);
            info.set_visible(stack.visible_child_name().as_deref() == Some("card") && has_item);
            result_count.set_text(&format!(
                "{completed} de {} ejercicios realizados · {skipped} omitidos. Lo parcial también cuenta.",
                preview.exercises.len()
            ));
            timer_options.set_sensitive(has_item);
            clock.set_visible(has_item && preview.timer_seconds > 0);
            clock.set_text(&format!("{:02}:{:02}", remaining / 60, remaining % 60));
            timer_button.set_visible(has_item && preview.timer_seconds > 0);
            timer_button.set_label(if preview.started.is_some() {
                "Pausar · Espacio"
            } else if remaining == 0 {
                "Repetir temporizador · Espacio"
            } else {
                "Iniciar temporizador · Espacio"
            });
            timer_message.set_text(if has_item && preview.timer_seconds > 0 && remaining == 0 {
                "Tiempo terminado. Avanza cuando estés listo."
            } else {
                ""
            });
        }
    ));
    begin.connect_clicked(glib::clone!(
        #[strong]
        initializing,
        #[strong]
        persist,
        #[strong]
        progress_signature,
        #[strong]
        state,
        #[strong]
        routines,
        #[strong]
        repo,
        #[weak]
        notes,
        #[weak]
        pain,
        #[weak]
        effort,
        #[strong]
        refresh,
        #[weak]
        selector,
        #[weak]
        stack,
        #[weak]
        exercise,
        move |_| {
            let Some(routine) = routines.get(selector.selected() as usize) else {
                return;
            };
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            let doc = repo
                .snapshot()
                .session_for_day(&routine.id, &today)
                .unwrap_or_else(|| crate::RegistroSesion::new(Some(routine)));
            progress_signature.borrow_mut().clear();
            let mut preview = state.borrow_mut();
            preview.key = routine.id.clone();
            preview.doc = Some(doc.clone());
            let mut categories = Vec::<String>::new();
            preview.exercises = doc
                .ejercicios
                .clone()
                .into_iter()
                .map(|e| {
                    let category_key =
                        format!("{}:{}", e.id.split(':').next().unwrap_or(""), e.categoria);
                    let color = if let Some(index) =
                        categories.iter().position(|key| key == &category_key)
                    {
                        index
                    } else {
                        categories.push(category_key);
                        categories.len() - 1
                    };
                    Exercise {
                        name: e.nombre,
                        plan: e.planeado,
                        category: e.categoria,
                        color,
                        done: e.estado == ExerciseState::Done,
                        skipped: e.estado == ExerciseState::Skipped,
                    }
                })
                .collect();
            preview.index = preview
                .exercises
                .iter()
                .position(|e| !e.done && !e.skipped)
                .unwrap_or(preview.exercises.len());
            preview.reset_timer();
            drop(preview);
            initializing.set(true);
            notes.set_text(&doc.notas);
            pain.set_selected(doc.dolor.map_or(0, |value| value as u32 + 1));
            effort.set_selected(doc.esfuerzo.map_or(0, |value| value as u32 + 1));
            initializing.set(false);
            persist();
            stack.set_visible_child_name("card");
            refresh();
            exercise.grab_focus();
        }
    ));
    done.connect_clicked(glib::clone!(
        #[weak]
        popover,
        #[weak]
        exercise,
        #[strong]
        persist,
        #[strong]
        state,
        #[strong]
        refresh,
        move |_| {
            let mut preview = state.borrow_mut();
            if preview.index >= preview.exercises.len() {
                preview.index = 0;
                preview.reset_timer();
            } else {
                preview.complete();
            }
            drop(preview);
            persist();
            refresh();
            popover.popdown();
            exercise.grab_focus();
        }
    ));
    skip.connect_clicked(glib::clone!(
        #[weak]
        popover,
        #[weak]
        exercise,
        #[strong]
        state,
        #[strong]
        persist,
        #[strong]
        refresh,
        move |_| {
            state.borrow_mut().skip();
            persist();
            refresh();
            popover.popdown();
            exercise.grab_focus();
        }
    ));
    back.connect_clicked(glib::clone!(
        #[weak]
        popover,
        #[weak]
        exercise,
        #[strong]
        state,
        #[strong]
        refresh,
        move |_| {
            state.borrow_mut().move_by(-1);
            refresh();
            popover.popdown();
            exercise.grab_focus();
        }
    ));
    next.connect_clicked(glib::clone!(
        #[weak]
        popover,
        #[weak]
        exercise,
        #[strong]
        state,
        #[strong]
        refresh,
        move |_| {
            state.borrow_mut().move_by(1);
            refresh();
            popover.popdown();
            exercise.grab_focus();
        }
    ));
    choose.connect_clicked(glib::clone!(
        #[strong]
        persist,
        #[strong]
        setup_refresh,
        #[weak]
        popover,
        #[strong]
        state,
        #[weak]
        stack,
        move |_| {
            if !persist() {
                return;
            }
            state.borrow_mut().reset_timer();
            popover.popdown();
            setup_refresh();
            stack.set_visible_child_name("setup");
        }
    ));
    timer_options.connect_selected_notify(glib::clone!(
        #[strong]
        state,
        #[strong]
        refresh,
        move |options| {
            let mut preview = state.borrow_mut();
            preview.timer_seconds = [0, 30, 60, 90, 120][options.selected() as usize];
            preview.reset_timer();
            drop(preview);
            refresh();
        }
    ));
    timer_button.connect_clicked(glib::clone!(
        #[weak]
        popover,
        #[weak]
        exercise,
        #[strong]
        state,
        #[strong]
        refresh,
        move |_| {
            state.borrow_mut().toggle_timer();
            refresh();
            popover.popdown();
            exercise.grab_focus();
        }
    ));
    finish.connect_clicked(glib::clone!(
        #[strong]
        state,
        #[strong]
        refresh,
        #[weak]
        popover,
        #[weak]
        stack,
        move |_| {
            state.borrow_mut().reset_timer();
            popover.popdown();
            stack.set_visible_child_name("result");
            refresh();
        }
    ));
    review.connect_clicked(glib::clone!(
        #[strong]
        state,
        #[strong]
        refresh,
        #[weak]
        stack,
        move |_| {
            let mut preview = state.borrow_mut();
            if preview.index >= preview.exercises.len() {
                preview.index = 0;
            }
            drop(preview);
            stack.set_visible_child_name("card");
            refresh();
        }
    ));
    notes.connect_changed(glib::clone!(
        #[strong]
        initializing,
        #[strong]
        state,
        #[strong]
        persist,
        move |entry| {
            if initializing.get() {
                return;
            }
            if let Some(doc) = state.borrow_mut().doc.as_mut() {
                doc.notas = entry.text().into();
            }
            persist();
        }
    ));
    pain.connect_selected_notify(glib::clone!(
        #[strong]
        initializing,
        #[strong]
        state,
        #[strong]
        persist,
        move |dd| {
            if initializing.get() {
                return;
            }
            if let Some(doc) = state.borrow_mut().doc.as_mut() {
                doc.dolor = if dd.selected() == 0 {
                    None
                } else {
                    Some((dd.selected() - 1) as f64)
                };
            }
            persist();
        }
    ));
    effort.connect_selected_notify(glib::clone!(
        #[strong]
        initializing,
        #[strong]
        state,
        #[strong]
        persist,
        move |dd| {
            if initializing.get() {
                return;
            }
            if let Some(doc) = state.borrow_mut().doc.as_mut() {
                doc.esfuerzo = if dd.selected() == 0 {
                    None
                } else {
                    Some((dd.selected() - 1) as f64)
                };
            }
            persist();
        }
    ));
    save.connect_clicked(glib::clone!(
        #[strong]
        state,
        #[strong]
        repo,
        #[weak]
        status,
        #[weak]
        window,
        move |_| {
            let Some(mut doc) = state.borrow().document() else {
                return;
            };
            doc.registrado_en = chrono::Local::now().to_rfc3339();
            let key = state.borrow().key.clone();
            match repo.save_session(&key, doc) {
                Ok(()) => {
                    state.borrow_mut().doc = None;
                    window.close();
                }
                Err(error) => {
                    status.set_text(&format!("No se pudo guardar: {error}"));
                    status.set_visible(true);
                }
            }
        }
    ));
    window.connect_close_request(glib::clone!(
        #[strong]
        persist,
        move |_| {
            if persist() {
                glib::Propagation::Proceed
            } else {
                glib::Propagation::Stop
            }
        }
    ));
    let keys = gtk::EventControllerKey::new();
    let timer_focus = gtk::EventControllerFocus::new();
    timer_options.add_controller(timer_focus.clone());
    let held = Rc::new(Cell::new(0u8));
    keys.connect_key_released(glib::clone!(
        #[strong]
        held,
        move |_, key, _, _| {
            let bit = match key {
                gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter => 1,
                gtk::gdk::Key::space => 2,
                gtk::gdk::Key::s | gtk::gdk::Key::S => 4,
                _ => 0,
            };
            held.set(held.get() & !bit);
        }
    ));
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    keys.connect_key_pressed(glib::clone!(
        #[strong]
        state,
        #[strong]
        refresh,
        #[strong]
        timer_focus,
        #[strong]
        held,
        #[weak]
        skip,
        #[weak]
        info_popover,
        #[weak]
        popover,
        #[weak]
        stack,
        #[weak]
        done,
        #[weak]
        window,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, key, _, modifiers| {
            if modifiers
                .intersects(gtk::gdk::ModifierType::CONTROL_MASK | gtk::gdk::ModifierType::ALT_MASK)
            {
                return glib::Propagation::Proceed;
            }
            if skip.has_focus()
                && matches!(
                    key,
                    gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter | gtk::gdk::Key::space
                )
            {
                return glib::Propagation::Proceed;
            }
            if key == gtk::gdk::Key::Escape {
                window.unfullscreen();
                return glib::Propagation::Proceed;
            }
            if stack.visible_child_name().as_deref() != Some("card")
                || timer_focus.contains_focus()
                || popover.is_visible()
                || info_popover.is_visible()
            {
                return glib::Propagation::Proceed;
            }
            let bit = match key {
                gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter => 1,
                gtk::gdk::Key::space => 2,
                gtk::gdk::Key::s | gtk::gdk::Key::S => 4,
                _ => 0,
            };
            if bit != 0 {
                if held.get() & bit != 0 {
                    return glib::Propagation::Stop;
                }
                held.set(held.get() | bit);
            }
            match key {
                gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter => done.emit_clicked(),
                gtk::gdk::Key::s | gtk::gdk::Key::S => skip.emit_clicked(),
                gtk::gdk::Key::space => {
                    state.borrow_mut().toggle_timer();
                    refresh();
                }
                gtk::gdk::Key::Left => {
                    state.borrow_mut().move_by(-1);
                    refresh();
                }
                gtk::gdk::Key::Right => {
                    state.borrow_mut().move_by(1);
                    refresh();
                }
                _ => return glib::Propagation::Proceed,
            }
            glib::Propagation::Stop
        }
    ));
    window.add_controller(keys);
    glib::timeout_add_local(
        Duration::from_millis(200),
        glib::clone!(
            #[weak]
            window,
            #[strong]
            refresh,
            #[upgrade_or]
            glib::ControlFlow::Break,
            move || {
                if !window.is_visible() {
                    return glib::ControlFlow::Break;
                }
                refresh();
                glib::ControlFlow::Continue
            }
        ),
    );
    window.connect_is_active_notify(glib::clone!(
        #[strong]
        state,
        #[strong]
        refresh,
        move |window| {
            if !window.is_active() && state.borrow().started.is_some() {
                state.borrow_mut().toggle_timer();
                refresh();
            }
        }
    ));
    window.present();
    parent.set_visible(false);
    window
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guided_work_survives_closing_and_updates_the_original_session() {
        let root = std::env::temp_dir().join(format!("bosu-guided-{}", glib::uuid_string_random()));
        let repo = Repository::open(root.clone()).unwrap();
        let routine = repo.snapshot().config.rutinas[0].clone();
        let mut doc = RegistroSesion::new(Some(&routine));
        doc.notas = "Conservar nota anterior".into();
        doc.dolor = Some(4.5);
        doc.ejercicios[0].estado = ExerciseState::Done;
        doc.ejercicios[0].realizado = "8 con ayuda".into();
        repo.save_session(&routine.id, doc.clone()).unwrap();
        let mut guided = Preview {
            exercises: doc
                .ejercicios
                .iter()
                .map(|e| Exercise {
                    name: e.nombre.clone(),
                    plan: e.planeado.clone(),
                    category: e.categoria.clone(),
                    color: 0,
                    done: e.estado == ExerciseState::Done,
                    skipped: e.estado == ExerciseState::Skipped,
                })
                .collect(),
            doc: Some(doc.clone()),
            key: routine.id.clone(),
            index: 1,
            ..Default::default()
        };
        guided.complete();
        guided.skip();
        guided.persist(&repo).unwrap();
        let reopened = Repository::open(root.clone()).unwrap();
        let restored = reopened
            .snapshot()
            .session_for_day(&routine.id, &doc.fecha)
            .unwrap();
        assert_eq!(restored.ejercicios_completados, 2);
        assert_eq!(restored.ejercicios[2].estado, ExerciseState::Skipped);
        assert_eq!(reopened.snapshot().sessions[0].ejercicios_completados, 1);
        assert_eq!(restored.dolor, Some(4.5));
        assert_eq!(restored.notas, doc.notas);
        assert_eq!(restored.ejercicios[0].realizado, "8 con ayuda");
        reopened.save_session(&routine.id, restored).unwrap();
        assert_eq!(reopened.snapshot().sessions.len(), 1);
        assert_eq!(reopened.snapshot().sessions[0].id, doc.id);
        assert_eq!(reopened.snapshot().sessions[0].ejercicios_completados, 2);
        std::fs::remove_dir_all(root).unwrap();
    }
    fn preview() -> Preview {
        Preview {
            exercises: (0..2)
                .map(|i| Exercise {
                    name: format!("Ejercicio {i}"),
                    plan: "10 repeticiones".into(),
                    category: "Prueba".into(),
                    color: i,
                    done: false,
                    skipped: false,
                })
                .collect(),
            ..Default::default()
        }
    }
    #[test]
    fn navigation_does_not_mark_exercises_and_completion_stops_at_end() {
        let mut preview = preview();
        preview.move_by(1);
        assert!(!preview.exercises[0].done);
        preview.move_by(-1);
        preview.complete();
        assert!(preview.exercises[0].done);
        assert_eq!(preview.index, 1);
        preview.complete();
        preview.complete();
        assert_eq!(preview.index, 2);
        assert!(preview.exercises.iter().all(|e| e.done));
        preview.move_by(-1);
        assert_eq!(preview.index, 1);
    }
    #[test]
    fn skipping_is_distinct_from_navigation_and_can_be_changed_to_done() {
        let mut preview = preview();
        preview.timer_seconds = 30;
        preview.toggle_timer();
        preview.skip();
        assert!(preview.exercises[0].skipped);
        assert!(!preview.exercises[0].done);
        assert_eq!(preview.index, 1);
        assert!(preview.started.is_none());
        assert_eq!(preview.elapsed(), Duration::ZERO);
        preview.move_by(-1);
        preview.complete();
        assert!(preview.exercises[0].done);
        assert!(!preview.exercises[0].skipped);
        preview.skip();
        preview.skip();
        assert_eq!(preview.index, preview.exercises.len());
        assert_eq!(preview.exercises.iter().filter(|e| e.done).count(), 1);
    }
    #[test]
    fn timer_is_optional_pauses_and_resets_on_navigation() {
        let mut preview = preview();
        preview.toggle_timer();
        assert!(preview.started.is_none());
        preview.timer_seconds = 30;
        preview.toggle_timer();
        assert!(preview.started.is_some());
        preview.toggle_timer();
        assert!(preview.started.is_none());
        preview.elapsed = Duration::from_secs(30);
        preview.toggle_timer();
        assert!(preview.elapsed().as_secs() < 30);
        preview.move_by(1);
        assert!(preview.started.is_none());
        assert_eq!(preview.elapsed(), Duration::ZERO);
        assert!(preview.exercises.iter().all(|e| !e.done));
    }
}
