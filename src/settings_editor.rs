use adw::prelude::*;
use adw::{PreferencesWindow, PreferencesPage, PreferencesGroup, ActionRow, ExpanderRow, EntryRow, ApplicationWindow, SpinRow};
use gtk::{Box, Button, Align, Orientation, glib};
use std::rc::Rc;
use std::cell::RefCell;
use crate::{Configuracion, Rutina, Categoria, Ejercicio};

pub fn show_settings_window<F>(
    parent: &ApplicationWindow,
    config: Rc<RefCell<Configuracion>>,
    on_save: F,
) where
    F: Fn() + 'static,
{
    let pref_window = PreferencesWindow::builder()
        .transient_for(parent)
        .modal(true)
        .title("Configuración")
        .build();

    let page_calendar = PreferencesPage::builder()
        .title("Datos")
        .icon_name("folder-symbolic")
        .build();

    let group_info = PreferencesGroup::builder()
        .title("Archivo de Configuración")
        .description("Los datos se guardan en JSON localmente.")
        .build();

    let row_path = ActionRow::builder()
        .title("Ubicación del Archivo")
        .subtitle("~/.local/share/bosu_config.json")
        .build();

    let btn_open = Button::builder()
        .label("Abrir Carpeta")
        .valign(Align::Center)
        .build();

    btn_open.connect_clicked(|_| {
        let path = glib::user_data_dir();
        let file = gtk::gio::File::for_path(path);
        if let Err(e) = gtk::gio::AppInfo::launch_default_for_uri(file.uri().as_str(), None::<&gtk::gio::AppLaunchContext>) {
            eprintln!("Error al abrir carpeta: {}", e);
        }
    });

    row_path.add_suffix(&btn_open);
    group_info.add(&row_path);
    page_calendar.add(&group_info);
    pref_window.add(&page_calendar);

    // ==========================================
    // PAGE: EDITOR
    // ==========================================
    let page_editor = PreferencesPage::builder()
        .title("Editor")
        .icon_name("document-edit-symbolic")
        .build();

    let temp_config = Rc::new(RefCell::new(config.borrow().clone()));
    let widgets_tracked = Rc::new(RefCell::new(Vec::<PreferencesGroup>::new()));

    render_editor(&page_editor, temp_config.clone(), widgets_tracked.clone());
    
    let btn_save = Button::builder()
        .label("Guardar Cambios")
        .css_classes(["suggested-action"])
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .halign(Align::Center)
        .build();

    let config_clone = config.clone();
    let temp_clone = temp_config.clone();
    let pref_window_clone = pref_window.clone();

    btn_save.connect_clicked(move |_| {
        *config_clone.borrow_mut() = temp_clone.borrow().clone();
        
        let mut config_path = glib::user_data_dir();
        config_path.push("bosu_config.json");
        if let Ok(json) = serde_json::to_string_pretty(&*config_clone.borrow()) {
            let _ = std::fs::write(&config_path, json);
        }

        on_save();
        pref_window_clone.close();
    });

    let box_bottom = Box::builder().orientation(Orientation::Vertical).build();
    box_bottom.append(&btn_save);
    let pg_bottom = PreferencesGroup::builder().build();
    pg_bottom.add(&box_bottom);
    page_editor.add(&pg_bottom);

    pref_window.add(&page_editor);
    pref_window.present();
}

fn render_editor(page: &PreferencesPage, temp_config: Rc<RefCell<Configuracion>>, widgets: Rc<RefCell<Vec<PreferencesGroup>>>) {
    for child in widgets.borrow().iter() {
        page.remove(child);
    }
    widgets.borrow_mut().clear();

    let config_val = temp_config.borrow().clone();

    for (rutina_idx, rutina) in config_val.rutinas.iter().enumerate() {
        let group_rutina = PreferencesGroup::builder()
            .title(&rutina.nombre)
            .build();

        // Nombre de la rutina
        let title_entry = EntryRow::builder()
            .title("Nombre de la Rutina")
            .text(&rutina.nombre)
            .build();
            
        let tc1 = temp_config.clone();
        title_entry.connect_changed(move |entry| {
            tc1.borrow_mut().rutinas[rutina_idx].nombre = entry.text().to_string();
        });
        group_rutina.add(&title_entry);

        // Meta semanal
        let spin_meta = SpinRow::with_range(1.0, 7.0, 1.0);
        spin_meta.set_title("Meta Semanal (Días)");
        spin_meta.set_value(rutina.meta_semanal as f64);
        
        let tc2 = temp_config.clone();
        spin_meta.connect_notify_local(Some("value"), move |spin, _| {
            // we have to cast spin back to SpinRow since the notify callback passes the object as glib::Object
            if let Some(s) = spin.downcast_ref::<SpinRow>() {
                tc2.borrow_mut().rutinas[rutina_idx].meta_semanal = s.value() as u32;
            }
        });
        group_rutina.add(&spin_meta);

        // Secciones
        for (cat_idx, cat) in rutina.secciones.iter().enumerate() {
            let expander = ExpanderRow::builder()
                .title(&cat.titulo)
                .subtitle("Sección de ejercicios")
                .build();

            let cat_title_entry = EntryRow::builder()
                .title("Nombre de Sección")
                .text(&cat.titulo)
                .build();
                
            let tc_cat = temp_config.clone();
            cat_title_entry.connect_changed(move |entry| {
                tc_cat.borrow_mut().rutinas[rutina_idx].secciones[cat_idx].titulo = entry.text().to_string();
            });
            expander.add_row(&cat_title_entry);

            for (ej_idx, ej) in cat.ejercicios.iter().enumerate() {
                let ej_group = PreferencesGroup::builder()
                    .title(&format!("Ejercicio {}", ej_idx + 1))
                    .margin_start(12)
                    .margin_end(12)
                    .margin_top(6)
                    .margin_bottom(6)
                    .build();
                    
                let ej_name = EntryRow::builder().title("Nombre").text(&ej.nombre).build();
                let tc_ej1 = temp_config.clone();
                ej_name.connect_changed(move |entry| {
                    tc_ej1.borrow_mut().rutinas[rutina_idx].secciones[cat_idx].ejercicios[ej_idx].nombre = entry.text().to_string();
                });
                ej_group.add(&ej_name);

                let ej_reps = EntryRow::builder().title("Repeticiones").text(&ej.repeticiones).build();
                let tc_ej2 = temp_config.clone();
                ej_reps.connect_changed(move |entry| {
                    tc_ej2.borrow_mut().rutinas[rutina_idx].secciones[cat_idx].ejercicios[ej_idx].repeticiones = entry.text().to_string();
                });
                ej_group.add(&ej_reps);

                let btn_del_ej = Button::builder().icon_name("user-trash-symbolic").css_classes(["destructive-action"]).valign(Align::Center).build();
                let tc_del_ej = temp_config.clone();
                let page_clone1 = page.clone();
                let widgets_clone1 = widgets.clone();
                btn_del_ej.connect_clicked(move |_| {
                    tc_del_ej.borrow_mut().rutinas[rutina_idx].secciones[cat_idx].ejercicios.remove(ej_idx);
                    render_editor(&page_clone1, tc_del_ej.clone(), widgets_clone1.clone());
                });
                
                let row_del_ej = ActionRow::builder().title("Eliminar Ejercicio").build();
                row_del_ej.add_suffix(&btn_del_ej);
                ej_group.add(&row_del_ej);

                expander.add_row(&ej_group);
            }

            let btn_add_ej = Button::builder().label("Agregar Ejercicio").margin_top(12).margin_bottom(12).margin_start(12).margin_end(12).build();
            let tc_add_ej = temp_config.clone();
            let page_clone2 = page.clone();
            let widgets_clone2 = widgets.clone();
            btn_add_ej.connect_clicked(move |_| {
                tc_add_ej.borrow_mut().rutinas[rutina_idx].secciones[cat_idx].ejercicios.push(Ejercicio {
                    nombre: "Nuevo Ejercicio".to_string(),
                    repeticiones: "10x3".to_string(),
                });
                render_editor(&page_clone2, tc_add_ej.clone(), widgets_clone2.clone());
            });
            expander.add_row(&ActionRow::builder().child(&btn_add_ej).build());

            let btn_del_cat = Button::builder().label("Eliminar Sección").css_classes(["destructive-action"]).margin_top(12).margin_bottom(12).margin_start(12).margin_end(12).build();
            let tc_del_cat = temp_config.clone();
            let page_clone3 = page.clone();
            let widgets_clone3 = widgets.clone();
            btn_del_cat.connect_clicked(move |_| {
                tc_del_cat.borrow_mut().rutinas[rutina_idx].secciones.remove(cat_idx);
                render_editor(&page_clone3, tc_del_cat.clone(), widgets_clone3.clone());
            });
            expander.add_row(&ActionRow::builder().child(&btn_del_cat).build());

            group_rutina.add(&expander);
        }

        let btn_add_cat = Button::builder().label("Agregar Sección").margin_top(12).margin_bottom(12).margin_start(12).margin_end(12).build();
        let tc_add_cat = temp_config.clone();
        let page_clone4 = page.clone();
        let widgets_clone4 = widgets.clone();
        btn_add_cat.connect_clicked(move |_| {
            let colores = ["group-blue", "group-green", "group-purple", "group-orange"];
            let idx = tc_add_cat.borrow().rutinas[rutina_idx].secciones.len() % colores.len();
            tc_add_cat.borrow_mut().rutinas[rutina_idx].secciones.push(Categoria {
                titulo: "Nueva Sección".to_string(),
                css_class: colores[idx].to_string(),
                ejercicios: vec![],
            });
            render_editor(&page_clone4, tc_add_cat.clone(), widgets_clone4.clone());
        });
        group_rutina.add(&ActionRow::builder().child(&btn_add_cat).build());
        
        let btn_del_rutina = Button::builder().label("Eliminar Rutina").css_classes(["destructive-action"]).margin_top(12).margin_bottom(12).margin_start(12).margin_end(12).build();
        let tc_del_rut = temp_config.clone();
        let page_clone5 = page.clone();
        let widgets_clone5 = widgets.clone();
        btn_del_rutina.connect_clicked(move |_| {
            tc_del_rut.borrow_mut().rutinas.remove(rutina_idx);
            render_editor(&page_clone5, tc_del_rut.clone(), widgets_clone5.clone());
        });
        group_rutina.add(&ActionRow::builder().child(&btn_del_rutina).build());

        page.add(&group_rutina);
        widgets.borrow_mut().push(group_rutina.clone());
    }

    let btn_add_rutina = Button::builder()
        .label("Agregar Nueva Rutina")
        .margin_top(24)
        .margin_bottom(24)
        .build();
        
    let tc_add_rut = temp_config.clone();
    let page_clone_r = page.clone();
    let widgets_clone_r = widgets.clone();
    btn_add_rutina.connect_clicked(move |_| {
        tc_add_rut.borrow_mut().rutinas.push(Rutina {
            id: format!("rutina_{}", glib::DateTime::now_utc().unwrap().to_unix()),
            nombre: "Nueva Rutina".to_string(),
            meta_semanal: 3,
            secciones: vec![],
        });
        render_editor(&page_clone_r, tc_add_rut.clone(), widgets_clone_r.clone());
    });
    
    let box_btn_r = Box::builder().orientation(Orientation::Vertical).build();
    box_btn_r.append(&btn_add_rutina);
    let pg_btn = PreferencesGroup::builder().build();
    pg_btn.add(&box_btn_r);
    
    page.add(&pg_btn);
    widgets.borrow_mut().push(pg_btn.clone());
}
