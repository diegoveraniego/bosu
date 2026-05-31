use adw::prelude::*;
use adw::{Application, ApplicationWindow, HeaderBar, ToolbarView, ActionRow, PreferencesGroup, ViewStack, ViewSwitcher, Banner, PreferencesWindow, PreferencesPage, StatusPage};
use gtk::{Box, Label, CheckButton, Scale, Entry, Button, Orientation, Align, ScrolledWindow, glib, Grid};
use serde::{Deserialize, Serialize};
use chrono::{Local, Duration, Datelike};
use std::fs::{File, OpenOptions};
use std::io::{Write, BufRead, BufReader};
use std::rc::Rc;
use std::cell::RefCell;

mod settings_editor;

const APP_ID: &str = "org.diego.Bosu";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegistroSesion {
    pub fecha: String,
    pub nivel_molestia: f64,
    pub notas: String,
    pub ejercicios_completados: usize,
    pub ejercicios_totales: usize,
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
            rutinas: vec![
                Rutina {
                    id: "default_rutina_1".to_string(),
                    nombre: "Rutina Principal".to_string(),
                    meta_semanal: 3,
                    secciones: vec![
                Categoria {
                    titulo: "Fase 1: Activación y Movilidad".to_string(),
                    css_class: "group-blue".to_string(),
                    ejercicios: vec![
                        Ejercicio { nombre: "Hundir cabeza y pera en almohada".to_string(), repeticiones: "10x3".to_string() },
                        Ejercicio { nombre: "Hundir cabeza en almohada".to_string(), repeticiones: "10x3".to_string() },
                        Ejercicio { nombre: "Pegado a pared brazos en T".to_string(), repeticiones: "10x3".to_string() },
                        Ejercicio { nombre: "Estiramiento rodillo brazos en pared".to_string(), repeticiones: "10x3".to_string() },
                        Ejercicio { nombre: "Estiramientos en colchoneta (yoga)".to_string(), repeticiones: "10x3".to_string() },
                    ],
                },
                Categoria {
                    titulo: "Fase 2: Fortalecimiento Escapular".to_string(),
                    css_class: "group-green".to_string(),
                    ejercicios: vec![
                        Ejercicio { nombre: "Bandita en Y".to_string(), repeticiones: "10x3".to_string() },
                        Ejercicio { nombre: "Bandita en Y hacia abajo".to_string(), repeticiones: "10x3".to_string() },
                        Ejercicio { nombre: "Bandita hacia el piso en T".to_string(), repeticiones: "10x3".to_string() },
                        Ejercicio { nombre: "Bandita cruzada".to_string(), repeticiones: "10x3".to_string() },
                        Ejercicio { nombre: "Bandita cruzada con mano contr. arriba".to_string(), repeticiones: "10x3".to_string() },
                        Ejercicio { nombre: "Levantar mancuernas de 2kg".to_string(), repeticiones: "10x3".to_string() },
                        Ejercicio { nombre: "Superman".to_string(), repeticiones: "10x3".to_string() },
                    ],
                },
                Categoria {
                    titulo: "Fase 3: Estabilidad de Pie".to_string(),
                    css_class: "group-purple".to_string(),
                    ejercicios: vec![
                        Ejercicio { nombre: "Bandita hacia el pecho".to_string(), repeticiones: "10x3".to_string() },
                        Ejercicio { nombre: "Banda hacia abajo con brazos rectos".to_string(), repeticiones: "10x3".to_string() },
                        Ejercicio { nombre: "Banda hacia abajo (brazo en ángulo recto)".to_string(), repeticiones: "10x3".to_string() },
                        Ejercicio { nombre: "Bandita ángulo recto int. (por brazo)".to_string(), repeticiones: "10x3".to_string() },
                        Ejercicio { nombre: "Bandita ángulo recto ext. (por brazo)".to_string(), repeticiones: "10x3".to_string() },
                    ],
                },
                Categoria {
                    titulo: "Fase 4: Resistencia (TRX)".to_string(),
                    css_class: "group-orange".to_string(),
                    ejercicios: vec![
                        Ejercicio { nombre: "Codos pegados al cuerpo, hombros relajados".to_string(), repeticiones: "10x3".to_string() },
                        Ejercicio { nombre: "Codos separados (T), hombros relajados".to_string(), repeticiones: "10x3".to_string() },
                        Ejercicio { nombre: "Estiramiento rodillo lateral".to_string(), repeticiones: "10x3".to_string() },
                    ],
                },
                    ],
                }
            ],
            ultima_rutina_seleccionada: None,
        }
    }
}

fn main() {
    let app = Application::builder().application_id(APP_ID).build();
    
    app.connect_startup(|_| {
        let display = gtk::gdk::Display::default().expect("No se pudo conectar al Display");
        
        // Agregar nuestra carpeta personalizada de iconos SVG.
        // Usamos el directorio del binario en vez de current_dir(),
        // para que funcione tanto desde terminal como desde el menú de GNOME.
        let icon_theme = gtk::IconTheme::for_display(&display);
        if let Ok(exe_path) = std::env::current_exe() {
            // En desarrollo: <proyecto>/target/release/bosu → busca en src/icons
            if let Some(exe_dir) = exe_path.parent() {
                let dev_icons = exe_dir
                    .join("../../src/icons");
                if dev_icons.exists() {
                    icon_theme.add_search_path(&dev_icons);
                }
            }
        }
        // El ícono principal (org.diego.Bosu.svg) ya está en
        // ~/.local/share/icons/hicolor/scalable/apps/ — GTK lo encuentra solo.

        let provider = gtk::CssProvider::new();
        // Inyectamos CSS para los gráficos y widgets custom nativos
        provider.load_from_string(
            ".heatmap-box { min-width: 42px; min-height: 42px; border-radius: 8px; margin: 4px; }
             .heatmap-0 { background-color: alpha(@theme_fg_color, 0.05); }
             .heatmap-1 { background-color: #a1dab4; }
             .heatmap-2 { background-color: #41b6c4; }
             .heatmap-3 { background-color: #2c7fb8; }
             .heatmap-4 { background-color: #253494; }
             
             .circle-box { min-width: 40px; min-height: 40px; border-radius: 50%; background-color: alpha(@theme_fg_color, 0.1); margin: 4px; }
             .circle-box.done { background-color: @success_color; color: white; }
             
             .stat-card { background-color: @card_bg_color; border-radius: 12px; padding: 16px; margin: 6px; }
             .bar-chart-bar { background-color: @warning_color; border-radius: 4px 4px 0 0; min-width: 24px; margin: 2px; }

             /* Colores de acentuación para los grupos de ejercicios */
             .group-blue list { background-color: alpha(@accent_bg_color, 0.20); }
             .group-green list { background-color: alpha(@success_color, 0.20); }
             .group-purple list { background-color: alpha(#c061cb, 0.20); }
             .group-orange list { background-color: alpha(@warning_color, 0.20); }
             "
        );
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let mut history: Vec<RegistroSesion> = Vec::new();
    let mut history_path = glib::user_data_dir();
    history_path.push("bosu_history.jsonl");

    if let Ok(file) = File::open(&history_path) {
        let reader = BufReader::new(file);
        for line in reader.lines().flatten() {
            if let Ok(registro) = serde_json::from_str::<RegistroSesion>(&line) {
                history.push(registro);
            }
        }
    }

    let mut config_path = glib::user_data_dir();
    config_path.push("bosu_config.json");
    
    let config = if let Ok(data) = std::fs::read_to_string(&config_path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        let def = Configuracion::default();
        std::fs::write(&config_path, serde_json::to_string_pretty(&def).unwrap()).unwrap();
        def
    };

    let config_rc = Rc::new(RefCell::new(config));

    let mut banner_title = String::new();
    let mut banner_is_error = false;
    let mut show_banner = false;

    if history.len() >= 3 {
        let last_3: Vec<&RegistroSesion> = history.iter().rev().take(3).collect();
        if last_3.iter().all(|r| r.nivel_molestia >= 7.0) {
            banner_title = "Alerta: 3 días seguidos con dolor alto. Consulta a tu kinesiólogo.".to_string();
            banner_is_error = true;
            show_banner = true;
        }
    }
    
    if !show_banner && history.len() >= 5 {
        let last_5: Vec<&RegistroSesion> = history.iter().rev().take(5).collect();
        if last_5.iter().all(|r| r.nivel_molestia <= 3.0) {
            banner_title = "Progresión: 5 días sin dolor. Considera subir peso o repeticiones.".to_string();
            show_banner = true;
        }
    }

    let banner = Banner::builder()
        .title(&banner_title)
        .revealed(show_banner)
        .build();

    if banner_is_error {
        banner.add_css_class("error");
    }

    let hoy_date = Local::now().date_naive();
    let current_weekday = hoy_date.weekday().number_from_monday();

    let content_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(18)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();

    populate_home_view(&content_box, config_rc.clone());

    let hoy_wrapper = Box::builder().orientation(Orientation::Vertical).build();
    hoy_wrapper.append(&banner);
    
    let scrolled_hoy = ScrolledWindow::builder().child(&content_box).vexpand(true).build();
    hoy_wrapper.append(&scrolled_hoy);

    // =========================================================================
    // VISTA "PROGRESO" (DASHBOARD)
    // =========================================================================
    let progress_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(32) // Espaciado mayor entre secciones
        .margin_top(24)
        .margin_start(24)
        .margin_end(24)
        .margin_bottom(36)
        .build();

    let total_sesiones = history.len();
    let dolor_promedio = if total_sesiones > 0 {
        history.iter().map(|r| r.nivel_molestia).sum::<f64>() / total_sesiones as f64
    } else { 0.0 };

    // --- 1. STAT CARDS ---
    let stats_grid = Grid::builder()
        .column_spacing(12)
        .row_spacing(12)
        .column_homogeneous(true)
        .build();

    let create_stat_card = |titulo: &str, valor: &str, icono: Option<&str>| -> Box {
        let card = Box::builder()
            .orientation(Orientation::Vertical)
            .css_classes(["stat-card"])
            .build();

        let val_box = Box::builder().orientation(Orientation::Horizontal).spacing(6).build();
        let val_label = Label::builder()
            .label(&format!("<span size='xx-large' weight='bold'>{}</span>", valor))
            .use_markup(true)
            .halign(Align::Start)
            .build();
        val_box.append(&val_label);

        if let Some(icn) = icono {
            let img = gtk::Image::from_icon_name(icn);
            img.add_css_class("warning");
            val_box.append(&img);
        }

        let desc_label = Label::builder()
            .label(titulo)
            .css_classes(["dim-label"])
            .halign(Align::Start)
            .margin_top(4)
            .build();

        card.append(&val_box);
        card.append(&desc_label);
        card
    };

    let card1 = create_stat_card("sesiones totales", &total_sesiones.to_string(), None);
    let card2 = create_stat_card("dolor promedio", &format!("{:.1}", dolor_promedio), None);
    
    // Calcular Racha (días consecutivos recientes)
    let racha = total_sesiones; // Placeholder simple para la racha por ahora
    let card3 = create_stat_card("racha actual", &racha.to_string(), Some("emblem-important-symbolic"));

    stats_grid.attach(&card1, 0, 0, 1, 1);
    stats_grid.attach(&card2, 1, 0, 1, 1);
    stats_grid.attach(&card3, 0, 1, 1, 1);
    
    progress_box.append(&stats_grid);

    // --- 2. ESTA SEMANA (Círculos) ---
    let sect1_title = Label::builder().label("<b>ESTA SEMANA</b>").use_markup(true).css_classes(["dim-label"]).halign(Align::Start).build();
    progress_box.append(&sect1_title);

    let semana_box = Box::builder().orientation(Orientation::Horizontal).spacing(8).halign(Align::Center).build();
    let dias_semana = ["Lun", "Mar", "Mié", "Jue", "Vie", "Sáb", "Dom"];
    
    for (i, nombre_dia) in dias_semana.iter().enumerate() {
        let col = Box::builder().orientation(Orientation::Vertical).spacing(6).build();
        
        let circulo = Box::builder()
            .css_classes(["circle-box"])
            .halign(Align::Center)
            .valign(Align::Center)
            .build();
        
        // Simular que chequeamos si hay registro en los últimos 7 días
        // Calculamos la fecha de ese día de la semana actual
        let diff = (i as i32) - (current_weekday as i32) + 1; // current_weekday 1-7
        let dia_obj = hoy_date + Duration::days(diff as i64);
        let dia_str = dia_obj.format("%Y-%m-%d").to_string();

        if let Some(sesion) = history.iter().find(|r| r.fecha.starts_with(&dia_str)) {
            circulo.add_css_class("done");
            let porcentaje = if sesion.ejercicios_totales > 0 {
                sesion.ejercicios_completados as f64 / sesion.ejercicios_totales as f64
            } else {
                1.0
            };
            circulo.set_opacity(porcentaje.max(0.15));
        }

        let label = Label::builder().label(*nombre_dia).css_classes(["dim-label"]).build();
        col.append(&circulo);
        col.append(&label);
        semana_box.append(&col);
    }
    progress_box.append(&semana_box);

    // --- 3. HEATMAP ---
    let sect2_title = Label::builder().label("<b>ÚLTIMAS 5 SEMANAS</b>").use_markup(true).css_classes(["dim-label"]).halign(Align::Start).build();
    progress_box.append(&sect2_title);

    let heatmap_grid = Grid::builder().halign(Align::Center).row_spacing(4).column_spacing(4).build();
    let dias_a_mostrar = 35; 
    let fecha_inicio = hoy_date - Duration::days(dias_a_mostrar - 1);

    for i in 0..dias_a_mostrar {
        let fecha_actual = fecha_inicio + Duration::days(i);
        let fecha_str = fecha_actual.format("%Y-%m-%d").to_string();

        let mut clase_intensidad = "heatmap-0";
        let mut tooltip = format!("{} - Sin registro", fecha_str);

        if let Some(sesion) = history.iter().find(|r| r.fecha.starts_with(&fecha_str)) {
            let porcentaje = sesion.ejercicios_completados as f64 / sesion.ejercicios_totales as f64;
            if porcentaje > 0.8 { clase_intensidad = "heatmap-4"; } 
            else if porcentaje > 0.5 { clase_intensidad = "heatmap-3"; } 
            else if porcentaje > 0.2 { clase_intensidad = "heatmap-2"; } 
            else { clase_intensidad = "heatmap-1"; }
            tooltip = format!("{} - {}% completado (Molestia: {})", fecha_str, (porcentaje * 100.0) as i32, sesion.nivel_molestia);
        }

        let cell = Box::builder().css_classes(["heatmap-box", clase_intensidad]).tooltip_text(&tooltip).build();
        let columna = (i % 7) as i32;
        let fila = (i / 7) as i32;
        heatmap_grid.attach(&cell, columna, fila, 1, 1);
    }
    progress_box.append(&heatmap_grid);

    // --- 4. GRÁFICO BARRAS: DOLOR VS MEJORA ---
    let sect3_title = Label::builder().label("<b>DOLOR (ÚLTIMAS 7 SESIONES)</b>").use_markup(true).css_classes(["dim-label"]).halign(Align::Start).build();
    progress_box.append(&sect3_title);

    let bar_container = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .halign(Align::Center)
        .height_request(100)
        .build();

    let ultimas_7: Vec<&RegistroSesion> = history.iter().rev().take(7).collect();
    
    for i in 0..7 {
        let box_wrap = Box::builder().orientation(Orientation::Vertical).valign(Align::End).build();
        let bar = Box::builder().css_classes(["bar-chart-bar"]).build();
        
        if i < ultimas_7.len() {
            let dolor = ultimas_7[i].nivel_molestia;
            bar.set_height_request((dolor * 10.0) as i32);
            bar.set_tooltip_text(Some(&format!("Dolor: {}", dolor)));
            if dolor >= 7.0 {
                bar.add_css_class("error");
            } else if dolor <= 3.0 {
                bar.add_css_class("success");
            }
        } else {
            bar.set_height_request(0);
        }
        
        box_wrap.append(&bar);
        bar_container.append(&box_wrap);
    }
    progress_box.append(&bar_container);

    let scrolled_progreso = ScrolledWindow::builder().child(&progress_box).vexpand(true).build();

    // =========================================================================

    let view_stack = ViewStack::builder().build();
    view_stack.add_titled_with_icon(&hoy_wrapper, Some("hoy"), "Hoy", "view-list-symbolic");
    view_stack.add_titled_with_icon(&scrolled_progreso, Some("progreso"), "Progreso", "charge-symbolic");

    let view_switcher = ViewSwitcher::builder()
        .stack(&view_stack)
        .policy(adw::ViewSwitcherPolicy::Narrow)
        .build();

    let header_bar = HeaderBar::builder()
        .title_widget(&view_switcher)
        .build();

    let btn_settings = Button::builder()
        .icon_name("emblem-system-symbolic")
        .build();

    header_bar.pack_end(&btn_settings);

    let toolbar_view = ToolbarView::builder().build();
    toolbar_view.add_top_bar(&header_bar);
    toolbar_view.set_content(Some(&view_stack));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Bosu")
        .icon_name(APP_ID)   // ← le dice a GNOME qué ícono mostrar en el dock
        .default_width(450)
        .default_height(800)
        .content(&toolbar_view)
        .build();

    // Lógica para abrir Ventana de Preferencias
    let content_box_clone = content_box.clone();
    let config_clone = config_rc.clone();
    btn_settings.connect_clicked(glib::clone!(#[weak] window, move |_| {
        let box_cb = content_box_clone.clone();
        let cfg_cb = config_clone.clone();
        settings_editor::show_settings_window(&window, cfg_cb.clone(), move || {
            populate_home_view(&box_cb, cfg_cb.clone());
        });
    }));

    window.present();
}

fn populate_home_view(content_box: &Box, config_rc: Rc<RefCell<Configuracion>>) {
    // Limpiar el contenedor
    while let Some(child) = content_box.first_child() {
        content_box.remove(&child);
    }

    let config = config_rc.borrow().clone();

    let mut options = Vec::new();
    options.push("Día de Descanso".to_string());
    for r in &config.rutinas {
        options.push(r.nombre.clone());
    }
    let string_list = gtk::StringList::new(&options.iter().map(|s| s.as_str()).collect::<Vec<&str>>());
    
    let dropdown = gtk::DropDown::builder()
        .model(&string_list)
        .valign(gtk::Align::Center)
        .build();

    let mut selected_idx = 0;
    if let Some(ref sel_id) = config.ultima_rutina_seleccionada {
        if let Some(pos) = config.rutinas.iter().position(|r| r.id == *sel_id) {
            selected_idx = (pos + 1) as u32;
        }
    }
    dropdown.set_selected(selected_idx);

    let selector_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .halign(Align::Center)
        .margin_bottom(12)
        .build();
    selector_box.append(&Label::builder().label("Rutina de hoy:").build());
    selector_box.append(&dropdown);
    content_box.append(&selector_box);

    let sections_container = Box::builder().orientation(Orientation::Vertical).spacing(18).build();
    content_box.append(&sections_container);

    let config_clone = config_rc.clone();
    let sections_container_clone = sections_container.clone();

    let render_sections = Rc::new(move |selected_idx: u32, container: &Box| {
        while let Some(child) = container.first_child() {
            container.remove(&child);
        }

        if selected_idx == 0 {
            let status = StatusPage::builder()
                .icon_name("non-emergency-healthcare-symbolic")
                .title("¡Hoy es día de descanso!")
                .description("Tus músculos también necesitan recuperación. Nos vemos mañana.")
                .build();
            container.append(&status);
            return;
        }

        let cfg = config_clone.borrow();
        let rutina = &cfg.rutinas[(selected_idx - 1) as usize];

        let mut check_buttons: Vec<CheckButton> = Vec::new();
        let mut total_ejercicios = 0;

        for categoria in &rutina.secciones {
            let group = PreferencesGroup::builder().build();
            group.add_css_class(&categoria.css_class);

            for ej in &categoria.ejercicios {
                let check = CheckButton::builder().valign(Align::Center).build();
                check_buttons.push(check.clone());
                total_ejercicios += 1;

                let row = ActionRow::builder()
                    .title(&ej.nombre)
                    .subtitle(&ej.repeticiones)
                    .activatable_widget(&check)
                    .build();
                
                row.add_prefix(&check);
                group.add(&row);
            }
            container.append(&group);
        }

        let eval_group = PreferencesGroup::builder().margin_top(12).build();
        let scale_label = Label::builder().label("Nivel de dolor / esfuerzo").halign(Align::Start).margin_bottom(6).build();
        let scale = Scale::with_range(Orientation::Horizontal, 1.0, 10.0, 1.0);
        scale.set_value(5.0);
        scale.set_draw_value(true);
        let notes_entry = Entry::builder().placeholder_text("Ej. Me dolió el hombro derecho...").build();
        let save_button = Button::builder().label("Guardar Sesión").css_classes(["suggested-action"]).margin_top(12).build();

        save_button.connect_clicked(glib::clone!(
            #[weak] scale, 
            #[weak] notes_entry, 
            #[strong] check_buttons,
            move |btn| {
                let mut completados = 0;
                for cb in &check_buttons {
                    if cb.is_active() { completados += 1; }
                }

                let registro = RegistroSesion {
                    fecha: Local::now().format("%Y-%m-%d").to_string(),
                    nivel_molestia: scale.value(),
                    notas: notes_entry.text().to_string(),
                    ejercicios_completados: completados,
                    ejercicios_totales: total_ejercicios,
                };

                if let Ok(json) = serde_json::to_string(&registro) {
                    let mut path = glib::user_data_dir();
                    path.push("bosu_history.jsonl");
                    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
                        let _ = writeln!(file, "{}", json);
                    }
                    
                    notes_entry.set_text("");
                    scale.set_value(5.0);
                    for cb in &check_buttons { cb.set_active(false); }

                    btn.set_label("¡Guardado!");
                    let btn_clone = btn.clone();
                    glib::timeout_add_seconds_local(2, move || {
                        btn_clone.set_label("Guardar Sesión");
                        glib::ControlFlow::Break
                    });
                }
            }
        ));

        let eval_box = Box::builder().orientation(Orientation::Vertical).spacing(12).margin_top(12).margin_bottom(12).margin_start(12).margin_end(12).build();
        eval_box.append(&scale_label);
        eval_box.append(&scale);
        eval_box.append(&notes_entry);
        eval_box.append(&save_button);
        eval_group.add(&ActionRow::builder().child(&eval_box).build());
        container.append(&eval_group);
    });

    render_sections(selected_idx, &sections_container);

    let render_sections_cb = render_sections.clone();
    let cfg_save = config_rc.clone();
    dropdown.connect_selected_notify(move |dd| {
        let idx = dd.selected();
        render_sections_cb(idx, &sections_container_clone);
        
        if idx > 0 {
            let id = cfg_save.borrow().rutinas[(idx - 1) as usize].id.clone();
            cfg_save.borrow_mut().ultima_rutina_seleccionada = Some(id);
        } else {
            cfg_save.borrow_mut().ultima_rutina_seleccionada = None;
        }
        
        let mut path = glib::user_data_dir();
        path.push("bosu_config.json");
        if let Ok(json) = serde_json::to_string_pretty(&*cfg_save.borrow()) {
            let _ = std::fs::write(path, json);
        }
    });
}
