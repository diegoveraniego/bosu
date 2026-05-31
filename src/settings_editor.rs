use adw::prelude::*;
use adw::{PreferencesWindow, PreferencesPage, PreferencesGroup, ActionRow, ExpanderRow, EntryRow, ApplicationWindow};
use gtk::{Box, Button, Align, Orientation, glib};
use std::rc::Rc;
use std::cell::RefCell;
use crate::{Configuracion, Categoria, Ejercicio};

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
        .title("Configuración de Rutinas")
        .build();

    let page_calendar = PreferencesPage::builder()
        .title("Datos Privados")
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
    // PAGE: EDITOR DE RUTINAS
    // ==========================================
    let page_editor = PreferencesPage::builder()
        .title("Editor de Rutinas")
        .icon_name("document-edit-symbolic")
        .build();

    let editor_group = PreferencesGroup::builder()
        .title("Mis Rutinas")
        .build();

    let temp_config = Rc::new(RefCell::new(config.borrow().clone()));
    let widgets_tracked = Rc::new(RefCell::new(Vec::<gtk::Widget>::new()));

    render_editor(&editor_group, temp_config.clone(), widgets_tracked.clone());
    
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
        // Copiar temp_config a config
        *config_clone.borrow_mut() = temp_clone.borrow().clone();
        
        // Guardar a archivo
        let mut config_path = glib::user_data_dir();
        config_path.push("bosu_config.json");
        if let Ok(json) = serde_json::to_string_pretty(&*config_clone.borrow()) {
            let _ = std::fs::write(&config_path, json);
        }

        on_save(); // Llama al callback para refrescar la UI principal
        pref_window_clone.close();
    });

    page_editor.add(&editor_group);
    
    let box_bottom = Box::builder().orientation(Orientation::Vertical).build();
    box_bottom.append(&btn_save);
    let pg_bottom = PreferencesGroup::builder().build();
    pg_bottom.add(&box_bottom);
    page_editor.add(&pg_bottom);

    pref_window.add(&page_editor);

    pref_window.present();
}

fn render_editor(group: &PreferencesGroup, temp_config: Rc<RefCell<Configuracion>>, widgets: Rc<RefCell<Vec<gtk::Widget>>>) {
    // Limpiar el contenedor
    for child in widgets.borrow().iter() {
        group.remove(child);
    }
    widgets.borrow_mut().clear();

    let config_val = temp_config.borrow().clone();

    for (cat_idx, cat) in config_val.rutinas.iter().enumerate() {
        let expander = ExpanderRow::builder()
            .title(&cat.titulo)
            .subtitle("Sección de ejercicios")
            .build();

        // Título de la sección
        let title_entry = EntryRow::builder()
            .title("Nombre")
            .text(&cat.titulo)
            .build();
            
        let tc = temp_config.clone();
        title_entry.connect_changed(move |entry| {
            tc.borrow_mut().rutinas[cat_idx].titulo = entry.text().to_string();
        });
        expander.add_row(&title_entry);

        // Lista de ejercicios
        for (ej_idx, ej) in cat.ejercicios.iter().enumerate() {
            let ej_group = PreferencesGroup::builder()
                .title(&format!("Ejercicio {}", ej_idx + 1))
                .margin_start(12)
                .margin_end(12)
                .margin_top(6)
                .margin_bottom(6)
                .build();
                
            let ej_name = EntryRow::builder()
                .title("Nombre")
                .text(&ej.nombre)
                .build();
            let tc2 = temp_config.clone();
            ej_name.connect_changed(move |entry| {
                tc2.borrow_mut().rutinas[cat_idx].ejercicios[ej_idx].nombre = entry.text().to_string();
            });
            ej_group.add(&ej_name);

            let ej_reps = EntryRow::builder()
                .title("Repeticiones")
                .text(&ej.repeticiones)
                .build();
            let tc3 = temp_config.clone();
            ej_reps.connect_changed(move |entry| {
                tc3.borrow_mut().rutinas[cat_idx].ejercicios[ej_idx].repeticiones = entry.text().to_string();
            });
            ej_group.add(&ej_reps);

            let btn_del_ej = Button::builder()
                .icon_name("user-trash-symbolic")
                .css_classes(["destructive-action"])
                .valign(Align::Center)
                .build();
                
            let tc_del_ej = temp_config.clone();
            let group_clone = group.clone();
            let widgets_clone = widgets.clone();
            btn_del_ej.connect_clicked(move |_| {
                tc_del_ej.borrow_mut().rutinas[cat_idx].ejercicios.remove(ej_idx);
                render_editor(&group_clone, tc_del_ej.clone(), widgets_clone.clone());
            });
            
            let row_del_ej = ActionRow::builder().title("Eliminar Ejercicio").build();
            row_del_ej.add_suffix(&btn_del_ej);
            ej_group.add(&row_del_ej);

            expander.add_row(&ej_group);
        }

        let btn_add_ej = Button::builder()
            .label("Agregar Ejercicio")
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();
            
        let tc_add_ej = temp_config.clone();
        let group_clone2 = group.clone();
        let widgets_clone2 = widgets.clone();
        btn_add_ej.connect_clicked(move |_| {
            tc_add_ej.borrow_mut().rutinas[cat_idx].ejercicios.push(Ejercicio {
                nombre: "Nuevo Ejercicio".to_string(),
                repeticiones: "10x3".to_string(),
            });
            render_editor(&group_clone2, tc_add_ej.clone(), widgets_clone2.clone());
        });
        expander.add_row(&ActionRow::builder().child(&btn_add_ej).build());

        let btn_del_cat = Button::builder()
            .label("Eliminar Sección")
            .css_classes(["destructive-action"])
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();
            
        let tc_del_cat = temp_config.clone();
        let group_clone3 = group.clone();
        let widgets_clone3 = widgets.clone();
        btn_del_cat.connect_clicked(move |_| {
            tc_del_cat.borrow_mut().rutinas.remove(cat_idx);
            render_editor(&group_clone3, tc_del_cat.clone(), widgets_clone3.clone());
        });
        expander.add_row(&ActionRow::builder().child(&btn_del_cat).build());

        group.add(&expander);
        widgets.borrow_mut().push(expander.upcast::<gtk::Widget>());
    }

    let btn_add_cat = Button::builder()
        .label("Agregar Nueva Sección")
        .margin_top(24)
        .margin_bottom(24)
        .build();
        
    let tc_add_cat = temp_config.clone();
    let group_clone4 = group.clone();
    let widgets_clone4 = widgets.clone();
    btn_add_cat.connect_clicked(move |_| {
        // Asignar un color ciclando los disponibles
        let colores = ["group-blue", "group-green", "group-purple", "group-orange"];
        let idx = tc_add_cat.borrow().rutinas.len() % colores.len();
        
        tc_add_cat.borrow_mut().rutinas.push(Categoria {
            titulo: "Nueva Sección".to_string(),
            css_class: colores[idx].to_string(),
            ejercicios: vec![],
        });
        render_editor(&group_clone4, tc_add_cat.clone(), widgets_clone4.clone());
    });
    
    let btn_row = ActionRow::builder().child(&btn_add_cat).build();
    group.add(&btn_row);
    widgets.borrow_mut().push(btn_row.upcast::<gtk::Widget>());
}
