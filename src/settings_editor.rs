use crate::{Categoria, Configuracion, Ejercicio, Rutina, storage::Repository};
use adw::prelude::*;
use adw::{
    ActionRow, ApplicationWindow, EntryRow, ExpanderRow, PreferencesGroup, PreferencesPage,
    PreferencesWindow, SpinRow,
};
use gtk::{Align, Box, Button, FileDialog, Orientation, gio, glib};
use std::{cell::RefCell, rc::Rc};

pub fn show_settings_window(
    parent: &ApplicationWindow,
    repo: Repository,
    on_save: Rc<dyn Fn()>,
    _overlay: adw::ToastOverlay,
) {
    let window = PreferencesWindow::builder()
        .transient_for(parent)
        .modal(true)
        .title(crate::i18n::tr("Preferencias"))
        .default_width(600)
        .default_height(700)
        .build();
    let toasts = window.clone();
    let data = repo.snapshot();
    let temp = Rc::new(RefCell::new(data.config));
    let preferences = Rc::new(RefCell::new(data.preferences));
    let page = PreferencesPage::builder()
        .title(crate::i18n::tr("Rutinas"))
        .icon_name("view-list-symbolic")
        .build();
    let tracked = Rc::new(RefCell::new(Vec::new()));
    render_editor(&page, temp.clone(), tracked);
    let message = gtk::Label::builder().wrap(true).xalign(0.).build();
    let save = Button::builder()
        .label(crate::i18n::tr("Guardar preferencias"))
        .css_classes(["suggested-action"])
        .margin_top(12)
        .margin_bottom(12)
        .build();
    let footer = PreferencesGroup::new();
    footer.add(&message);
    footer.add(&save);
    page.add(&footer);
    window.add(&page);
    let pref_page = PreferencesPage::builder()
        .title(crate::i18n::tr("Acompañamiento"))
        .icon_name("preferences-system-notifications-symbolic")
        .build();
    let reminder_group=PreferencesGroup::builder().title(crate::i18n::tr("Recordatorios a tu ritmo")).description(crate::i18n::tr("Opcionales y sin presión. Funcionan mientras Bosu está abierto; no se instala ningún servicio en segundo plano.")).build();
    let enable = adw::SwitchRow::builder()
        .title(crate::i18n::tr("Recordatorio diario"))
        .active(preferences.borrow().reminders)
        .build();
    reminder_group.add(&enable);
    let hour = SpinRow::with_range(0., 23., 1.);
    hour.set_title(&crate::i18n::tr("Hora"));
    hour.set_value(preferences.borrow().reminder_hour as f64);
    reminder_group.add(&hour);
    let minute = SpinRow::with_range(0., 59., 1.);
    minute.set_title(&crate::i18n::tr("Minutos"));
    minute.set_value(preferences.borrow().reminder_minute as f64);
    reminder_group.add(&minute);
    enable.connect_active_notify(glib::clone!(
        #[strong]
        preferences,
        move |r| preferences.borrow_mut().reminders = r.is_active()
    ));
    hour.connect_value_notify(glib::clone!(
        #[strong]
        preferences,
        move |r| preferences.borrow_mut().reminder_hour = r.value() as u32
    ));
    minute.connect_value_notify(glib::clone!(
        #[strong]
        preferences,
        move |r| preferences.borrow_mut().reminder_minute = r.value() as u32
    ));
    let snooze = Button::with_label(&crate::i18n::tr("Posponer recordatorios hasta mañana"));
    snooze.connect_clicked(glib::clone!(
        #[strong]
        preferences,
        #[weak]
        toasts,
        move |_| {
            preferences.borrow_mut().snooze_until = (chrono::Local::now().date_naive()
                + chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();
            toasts.add_toast(crate::toast(&crate::i18n::tr(
                "Pospuesto. Guarda las preferencias para aplicar el cambio.",
            )));
        }
    ));
    reminder_group.add(&snooze);
    pref_page.add(&reminder_group);
    let language_group=PreferencesGroup::builder().title(crate::i18n::tr("Idioma")).description(crate::i18n::tr("El cambio se aplica al reiniciar. Los nombres de tus ejercicios y tus notas se conservan.")).build();
    let language = crate::i18n::dropdown(&["Idioma del sistema", "Español", "English"]);
    language.set_selected(match preferences.borrow().language.as_str() {
        "es" => 1,
        "en" => 2,
        _ => 0,
    });
    language.connect_selected_notify(glib::clone!(
        #[strong]
        preferences,
        move |d| preferences.borrow_mut().language = match d.selected() {
            1 => "es",
            2 => "en",
            _ => "",
        }
        .into()
    ));
    let row = ActionRow::builder()
        .use_markup(false)
        .title(crate::i18n::tr("Idioma de la interfaz"))
        .build();
    row.add_suffix(&language);
    language_group.add(&row);
    pref_page.add(&language_group);
    let reminder_footer = PreferencesGroup::new();
    let reminder_save = Button::builder()
        .label(crate::i18n::tr("Guardar preferencias"))
        .css_classes(["suggested-action"])
        .build();
    let reminder_error = gtk::Label::builder().wrap(true).xalign(0.).build();
    reminder_footer.add(&reminder_error);
    reminder_footer.add(&reminder_save);
    pref_page.add(&reminder_footer);
    reminder_save.connect_clicked(glib::clone!(
        #[weak]
        save,
        move |_| save.emit_clicked()
    ));
    message.connect_label_notify(glib::clone!(
        #[weak]
        reminder_error,
        move |label| reminder_error.set_text(&label.label())
    ));
    window.add(&pref_page);
    save.connect_clicked(glib::clone!(
        #[strong]
        repo,
        #[strong]
        temp,
        #[strong]
        preferences,
        #[strong]
        on_save,
        #[weak]
        window,
        #[weak]
        message,
        move |_| {
            let config = temp.borrow().clone();
            let prefs = preferences.borrow().clone();
            match repo.update(|data| {
                data.config = config;
                data.preferences = prefs;
                Ok(())
            }) {
                Ok(()) => {
                    on_save();
                    window.close();
                }
                Err(e) => message.set_text(&crate::i18n::tr(&format!(
                    "No se pudo guardar. Tus cambios siguen abiertos: {e}"
                ))),
            }
        }
    ));
    let data_page = PreferencesPage::builder()
        .title(crate::i18n::tr("Datos"))
        .icon_name("folder-symbolic")
        .build();
    let group=PreferencesGroup::builder().title(crate::i18n::tr("Tus datos, en este equipo")).description(crate::i18n::tr("Sin cuentas ni telemetría. Los respaldos incluyen rutinas, sesiones, mapas y borradores.")).build();
    let location = ActionRow::builder()
        .use_markup(false)
        .title(crate::i18n::tr("Ubicación"))
        .subtitle(repo.root().display().to_string())
        .build();
    let open = Button::builder()
        .label(crate::i18n::tr("Abrir carpeta"))
        .valign(Align::Center)
        .build();
    location.add_suffix(&open);
    group.add(&location);
    open.connect_clicked(glib::clone!(
        #[strong]
        repo,
        #[weak]
        toasts,
        move |_| {
            let file = gio::File::for_path(repo.root());
            if let Err(e) = gio::AppInfo::launch_default_for_uri(
                file.uri().as_str(),
                None::<&gio::AppLaunchContext>,
            ) {
                toasts.add_toast(crate::toast(&crate::i18n::tr(&format!(
                    "No se pudo abrir la carpeta: {e}"
                ))));
            }
        }
    ));
    let export = Button::with_label(&crate::i18n::tr("Guardar respaldo…"));
    group.add(&export);
    export.connect_clicked(glib::clone!(
        #[strong]
        repo,
        #[weak]
        toasts,
        #[weak]
        window,
        move |_| {
            let dialog = FileDialog::builder()
                .title(crate::i18n::tr("Guardar respaldo completo"))
                .initial_name("bosu-respaldo.json")
                .build();
            let repo = repo.clone();
            let toasts = toasts.clone();
            dialog.save(Some(&window), gio::Cancellable::NONE, move |result| {
                if let Ok(file) = result {
                    let result = repo.export_file(&file);
                    toasts.add_toast(crate::toast(&crate::i18n::tr(&match result {
                        Ok(()) => "Respaldo guardado.".to_string(),
                        Err(e) => format!("No se pudo guardar el respaldo: {e}"),
                    })));
                }
            });
        }
    ));
    let import = Button::with_label(&crate::i18n::tr("Restaurar respaldo…"));
    group.add(&import);
    import.connect_clicked(glib::clone!(#[strong] repo,#[weak] toasts,#[strong] on_save,#[weak] window,move|_|{
        let dialog=FileDialog::builder().title(crate::i18n::tr("Elegir respaldo de Bosu")).build();let repo=repo.clone();let toasts=toasts.clone();let on_save=on_save.clone();let parent=window.clone();
        dialog.open(Some(&window),gio::Cancellable::NONE,move|result|{if let Ok(file)=result{let Some(path)=file.path()else{return;};match Repository::inspect_backup(&path){Err(e)=>toasts.add_toast(crate::toast(&crate::i18n::tr(&format!("No se pudo leer el respaldo: {e}")))),Ok(incoming)=>{
            let dialog=adw::MessageDialog::new(Some(&parent),Some(&crate::i18n::tr("¿Restaurar este respaldo?")),Some(&format!("Contiene {} sesiones, {} mapas independientes y {} borradores. Reemplazará tus datos actuales. Antes se guardará un respaldo de seguridad automático.",incoming.sessions.len(),incoming.body.len(),incoming.drafts.len())));
            dialog.add_responses(&[("cancel","Cancelar"),("restore","Restaurar")]);dialog.set_response_appearance("restore",adw::ResponseAppearance::Destructive);dialog.set_default_response(Some("cancel"));dialog.set_close_response("cancel");let parent=parent.clone();
            dialog.connect_response(Some("restore"),move|_,_|match repo.import(&path){Ok(previous)=>{toasts.add_toast(crate::toast(&crate::i18n::tr(&format!("Datos restaurados. Copia anterior: {}",previous.display()))));on_save();parent.close();},Err(e)=>toasts.add_toast(crate::toast(&crate::i18n::tr(&format!("No se pudo restaurar: {e}"))))});dialog.present();
        }}}});
    }));
    data_page.add(&group);
    window.add(&data_page);
    let history_page = PreferencesPage::builder()
        .title(crate::i18n::tr("Historial"))
        .icon_name("document-open-recent-symbolic")
        .build();
    let history_group=PreferencesGroup::builder().title(crate::i18n::tr("Sesiones y descansos")).description(crate::i18n::tr("Eliminar una sesión también elimina sus mapas vinculados. Deshacer restaura el conjunto.")).build();
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    let undo = Button::with_label(&crate::i18n::tr("Deshacer última eliminación"));
    history_group.add(&undo);
    history_group.add(&list);
    history_page.add(&history_group);
    window.add(&history_page);
    render_history(&list, &repo, on_save.clone(), toasts.clone());
    undo.connect_clicked(glib::clone!(
        #[strong]
        repo,
        #[strong]
        on_save,
        #[weak]
        toasts,
        #[weak]
        list,
        move |_| match repo.undo_delete() {
            Ok(()) => {
                on_save();
                render_history(&list, &repo, on_save.clone(), toasts.clone());
                toasts.add_toast(crate::toast(&crate::i18n::tr("Registro restaurado.")));
            }
            Err(e) => toasts.add_toast(crate::toast(&crate::i18n::tr(&e))),
        }
    ));
    window.present();
}
fn render_history(
    list: &gtk::ListBox,
    repo: &Repository,
    on_change: Rc<dyn Fn()>,
    toasts: PreferencesWindow,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let data = repo.snapshot();
    let mut ordered: Vec<_> = data.sessions.iter().collect();
    ordered.sort_by(|a, b| {
        b.fecha
            .cmp(&a.fecha)
            .then_with(|| b.registrado_en.cmp(&a.registrado_en))
    });
    for session in ordered {
        let row = ActionRow::builder()
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
        let delete = Button::builder()
            .icon_name("user-trash-symbolic")
            .tooltip_text(crate::i18n::tr("Eliminar registro · se puede deshacer"))
            .valign(Align::Center)
            .build();
        row.add_suffix(&delete);
        list.append(&row);
        let id = session.id.clone();
        delete.connect_clicked(glib::clone!(
            #[strong]
            repo,
            #[strong]
            on_change,
            #[strong]
            toasts,
            #[weak]
            list,
            move |_| match repo.delete_session(&id) {
                Ok(()) => {
                    on_change();
                    render_history(&list, &repo, on_change.clone(), toasts.clone());
                    let toast = adw::Toast::builder()
                        .use_markup(false)
                        .title(crate::i18n::tr("Registro eliminado"))
                        .button_label("Deshacer")
                        .timeout(8)
                        .build();
                    toast.connect_button_clicked(glib::clone!(
                        #[strong]
                        repo,
                        #[strong]
                        id,
                        #[strong]
                        on_change,
                        #[strong]
                        toasts,
                        #[weak]
                        list,
                        move |_| match repo.undo_delete_for(&id) {
                            Ok(()) => {
                                on_change();
                                render_history(&list, &repo, on_change.clone(), toasts.clone());
                            }
                            Err(e) => toasts.add_toast(crate::toast(&crate::i18n::tr(&e))),
                        }
                    ));
                    toasts.add_toast(toast);
                }
                Err(e) => toasts.add_toast(crate::toast(&crate::i18n::tr(&format!(
                    "No se pudo eliminar: {e}"
                )))),
            }
        ));
    }
}
fn render_editor(
    page: &PreferencesPage,
    temp_config: Rc<RefCell<Configuracion>>,
    widgets: Rc<RefCell<Vec<glib::WeakRef<PreferencesGroup>>>>,
) {
    for child in widgets.borrow().iter() {
        if let Some(child) = child.upgrade() {
            page.remove(&child);
        }
    }
    widgets.borrow_mut().clear();

    let config_val = temp_config.borrow().clone();

    for (rutina_idx, rutina) in config_val.rutinas.iter().enumerate() {
        let group_rutina = PreferencesGroup::builder().title(&rutina.nombre).build();

        // Nombre de la rutina
        let title_entry = EntryRow::builder()
            .use_markup(false)
            .title(crate::i18n::tr("Nombre de la Rutina"))
            .text(&rutina.nombre)
            .build();

        let tc1 = temp_config.clone();
        title_entry.connect_changed(move |entry| {
            tc1.borrow_mut().rutinas[rutina_idx].nombre = entry.text().to_string();
        });
        group_rutina.add(&title_entry);

        // Meta semanal
        let spin_meta = SpinRow::with_range(1.0, 7.0, 1.0);
        spin_meta.set_title(&crate::i18n::tr(if rutina.id == "default_rutina_1" {
            "Meta compartida de kine (días de práctica)"
        } else {
            "Meta semanal (días de práctica)"
        }));
        spin_meta.set_value(rutina.meta_semanal as f64);

        let tc2 = temp_config.clone();
        spin_meta.connect_notify_local(Some("value"), move |spin, _| {
            // we have to cast spin back to SpinRow since the notify callback passes the object as glib::Object
            if let Some(s) = spin.downcast_ref::<SpinRow>() {
                tc2.borrow_mut().rutinas[rutina_idx].meta_semanal = s.value() as u32;
            }
        });
        if !matches!(rutina.id.as_str(), "sesion_kine_1" | "sesion_kine_2") {
            group_rutina.add(&spin_meta);
        }

        // Secciones
        for (cat_idx, cat) in rutina.secciones.iter().enumerate() {
            let expander = ExpanderRow::builder()
                .use_markup(false)
                .title(&cat.titulo)
                .subtitle(crate::i18n::tr("Sección de ejercicios"))
                .build();

            let cat_title_entry = EntryRow::builder()
                .use_markup(false)
                .title(crate::i18n::tr("Nombre de Sección"))
                .text(&cat.titulo)
                .build();

            let tc_cat = temp_config.clone();
            cat_title_entry.connect_changed(move |entry| {
                tc_cat.borrow_mut().rutinas[rutina_idx].secciones[cat_idx].titulo =
                    entry.text().to_string();
            });
            expander.add_row(&cat_title_entry);

            for (ej_idx, ej) in cat.ejercicios.iter().enumerate() {
                let ej_group = PreferencesGroup::builder()
                    .title(crate::i18n::tr(&format!("Ejercicio {}", ej_idx + 1)))
                    .margin_start(12)
                    .margin_end(12)
                    .margin_top(6)
                    .margin_bottom(6)
                    .build();

                let ej_name = EntryRow::builder()
                    .use_markup(false)
                    .title(crate::i18n::tr("Nombre"))
                    .text(&ej.nombre)
                    .build();
                let tc_ej1 = temp_config.clone();
                ej_name.connect_changed(move |entry| {
                    tc_ej1.borrow_mut().rutinas[rutina_idx].secciones[cat_idx].ejercicios[ej_idx]
                        .nombre = entry.text().to_string();
                });
                ej_group.add(&ej_name);

                let ej_reps = EntryRow::builder()
                    .use_markup(false)
                    .title(crate::i18n::tr("Repeticiones"))
                    .text(&ej.repeticiones)
                    .build();
                let tc_ej2 = temp_config.clone();
                ej_reps.connect_changed(move |entry| {
                    tc_ej2.borrow_mut().rutinas[rutina_idx].secciones[cat_idx].ejercicios[ej_idx]
                        .repeticiones = entry.text().to_string();
                });
                ej_group.add(&ej_reps);

                let btn_del_ej = Button::builder()
                    .icon_name("user-trash-symbolic")
                    .css_classes(["destructive-action"])
                    .valign(Align::Center)
                    .build();
                let tc_del_ej = temp_config.clone();
                let page_clone1 = page.downgrade();
                let widgets_clone1 = widgets.clone();
                btn_del_ej.connect_clicked(move |_| {
                    tc_del_ej.borrow_mut().rutinas[rutina_idx].secciones[cat_idx]
                        .ejercicios
                        .remove(ej_idx);
                    if let Some(page) = page_clone1.upgrade() {
                        render_editor(&page, tc_del_ej.clone(), widgets_clone1.clone());
                    }
                });

                let row_del_ej = ActionRow::builder()
                    .use_markup(false)
                    .title(crate::i18n::tr("Eliminar Ejercicio"))
                    .build();
                row_del_ej.add_suffix(&btn_del_ej);
                ej_group.add(&row_del_ej);

                expander.add_row(&ej_group);
            }

            let btn_add_ej = Button::builder()
                .label(crate::i18n::tr("Agregar Ejercicio"))
                .margin_top(12)
                .margin_bottom(12)
                .margin_start(12)
                .margin_end(12)
                .build();
            let tc_add_ej = temp_config.clone();
            let page_clone2 = page.downgrade();
            let widgets_clone2 = widgets.clone();
            btn_add_ej.connect_clicked(move |_| {
                tc_add_ej.borrow_mut().rutinas[rutina_idx].secciones[cat_idx]
                    .ejercicios
                    .push(Ejercicio {
                        nombre: "Nuevo Ejercicio".to_string(),
                        repeticiones: "10x3".to_string(),
                    });
                if let Some(page) = page_clone2.upgrade() {
                    render_editor(&page, tc_add_ej.clone(), widgets_clone2.clone());
                }
            });
            expander.add_row(
                &ActionRow::builder()
                    .use_markup(false)
                    .child(&btn_add_ej)
                    .build(),
            );

            let btn_del_cat = Button::builder()
                .label(crate::i18n::tr("Eliminar Sección"))
                .css_classes(["destructive-action"])
                .margin_top(12)
                .margin_bottom(12)
                .margin_start(12)
                .margin_end(12)
                .build();
            let tc_del_cat = temp_config.clone();
            let page_clone3 = page.downgrade();
            let widgets_clone3 = widgets.clone();
            btn_del_cat.connect_clicked(move |_| {
                tc_del_cat.borrow_mut().rutinas[rutina_idx]
                    .secciones
                    .remove(cat_idx);
                if let Some(page) = page_clone3.upgrade() {
                    render_editor(&page, tc_del_cat.clone(), widgets_clone3.clone());
                }
            });
            expander.add_row(
                &ActionRow::builder()
                    .use_markup(false)
                    .child(&btn_del_cat)
                    .build(),
            );

            group_rutina.add(&expander);
        }

        let btn_add_cat = Button::builder()
            .label(crate::i18n::tr("Agregar Sección"))
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();
        let tc_add_cat = temp_config.clone();
        let page_clone4 = page.downgrade();
        let widgets_clone4 = widgets.clone();
        btn_add_cat.connect_clicked(move |_| {
            let colores = ["group-blue", "group-green", "group-purple", "group-orange"];
            let idx = tc_add_cat.borrow().rutinas[rutina_idx].secciones.len() % colores.len();
            tc_add_cat.borrow_mut().rutinas[rutina_idx]
                .secciones
                .push(Categoria {
                    titulo: "Nueva Sección".to_string(),
                    css_class: colores[idx].to_string(),
                    ejercicios: vec![],
                });
            if let Some(page) = page_clone4.upgrade() {
                render_editor(&page, tc_add_cat.clone(), widgets_clone4.clone());
            }
        });
        group_rutina.add(
            &ActionRow::builder()
                .use_markup(false)
                .child(&btn_add_cat)
                .build(),
        );

        let btn_del_rutina = Button::builder()
            .label(crate::i18n::tr("Eliminar Rutina"))
            .css_classes(["destructive-action"])
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();
        let tc_del_rut = temp_config.clone();
        let page_clone5 = page.downgrade();
        let widgets_clone5 = widgets.clone();
        btn_del_rutina.connect_clicked(move |_| {
            tc_del_rut.borrow_mut().rutinas.remove(rutina_idx);
            if let Some(page) = page_clone5.upgrade() {
                render_editor(&page, tc_del_rut.clone(), widgets_clone5.clone());
            }
        });
        group_rutina.add(
            &ActionRow::builder()
                .use_markup(false)
                .child(&btn_del_rutina)
                .build(),
        );

        page.add(&group_rutina);
        widgets.borrow_mut().push(group_rutina.downgrade());
    }

    let btn_add_rutina = Button::builder()
        .label(crate::i18n::tr("Agregar Nueva Rutina"))
        .margin_top(24)
        .margin_bottom(24)
        .build();

    let tc_add_rut = temp_config.clone();
    let page_clone_r = page.downgrade();
    let widgets_clone_r = widgets.clone();
    btn_add_rutina.connect_clicked(move |_| {
        tc_add_rut.borrow_mut().rutinas.push(Rutina {
            id: format!("rutina_{}", glib::DateTime::now_utc().unwrap().to_unix()),
            nombre: "Nueva Rutina".to_string(),
            meta_semanal: 3,
            secciones: vec![],
        });
        if let Some(page) = page_clone_r.upgrade() {
            render_editor(&page, tc_add_rut.clone(), widgets_clone_r.clone());
        }
    });

    let box_btn_r = Box::builder().orientation(Orientation::Vertical).build();
    box_btn_r.append(&btn_add_rutina);
    let pg_btn = PreferencesGroup::builder().build();
    pg_btn.add(&box_btn_r);

    page.add(&pg_btn);
    widgets.borrow_mut().push(pg_btn.downgrade());
}
