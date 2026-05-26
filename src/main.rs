use adw::prelude::*;
use adw::{Application, ApplicationWindow, HeaderBar, ToolbarView, ActionRow, PreferencesGroup, ViewStack, ViewSwitcher, Banner};
use gtk::{Box, Label, CheckButton, Scale, Entry, Button, Orientation, Separator, Align, ScrolledWindow, glib, Grid};
use serde::{Deserialize, Serialize};
use chrono::{Local, Duration};
use std::fs::{File, OpenOptions};
use std::io::{Write, BufRead, BufReader};

const APP_ID: &str = "org.diego.Bosu";

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RegistroSesion {
    fecha: String,
    nivel_molestia: f64,
    notas: String,
    ejercicios_completados: usize,
    ejercicios_totales: usize,
}

struct Ejercicio {
    nombre: &'static str,
    repeticiones: &'static str,
}

struct Categoria {
    titulo: &'static str,
    ejercicios: Vec<Ejercicio>,
}

fn main() {
    let app = Application::builder().application_id(APP_ID).build();
    
    // Inyectamos nuestro CSS personalizado para el Heatmap
    app.connect_startup(|_| {
        let provider = gtk::CssProvider::new();
        // Usamos colores verde-azulado (Estilo GitHub oscuro)
        provider.load_from_string(
            ".heatmap-box { min-width: 42px; min-height: 42px; border-radius: 8px; margin: 4px; }
             .heatmap-0 { background-color: alpha(@theme_fg_color, 0.05); }
             .heatmap-1 { background-color: #a1dab4; }
             .heatmap-2 { background-color: #41b6c4; }
             .heatmap-3 { background-color: #2c7fb8; }
             .heatmap-4 { background-color: #253494; }"
        );
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().expect("No se pudo conectar al Display"),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let mut history: Vec<RegistroSesion> = Vec::new();
    let mut path = glib::user_data_dir();
    path.push("bosu_history.jsonl");

    if let Ok(file) = File::open(&path) {
        let reader = BufReader::new(file);
        for line in reader.lines().flatten() {
            if let Ok(registro) = serde_json::from_str::<RegistroSesion>(&line) {
                history.push(registro);
            }
        }
    }

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

    let content_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(18)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();

    let main_title = Label::builder()
        .label("<b>Rutina Actual</b>")
        .use_markup(true)
        .halign(Align::Start)
        .build();
    content_box.append(&main_title);

    let rutina = vec![
        Categoria {
            titulo: "Acostado",
            ejercicios: vec![
                Ejercicio { nombre: "Hundir cabeza y pera en almohada", repeticiones: "10x3" },
                Ejercicio { nombre: "Hundir cabeza en almohada", repeticiones: "10x3" },
                Ejercicio { nombre: "Levantar mancuernas de 2kg", repeticiones: "10x3" },
                Ejercicio { nombre: "Bandita hacia el piso en T", repeticiones: "10x3" },
                Ejercicio { nombre: "Bandita en Y", repeticiones: "10x3" },
                Ejercicio { nombre: "Bandita en Y hacia abajo", repeticiones: "10x3" },
                Ejercicio { nombre: "Bandita cruzada", repeticiones: "10x3" },
                Ejercicio { nombre: "Bandita cruzada con mano contr. arriba", repeticiones: "10x3" },
                Ejercicio { nombre: "Superman", repeticiones: "10x3" },
                Ejercicio { nombre: "Estiramientos en colchoneta (yoga)", repeticiones: "10x3" },
            ],
        },
        Categoria {
            titulo: "De pie",
            ejercicios: vec![
                Ejercicio { nombre: "Bandita hacia el pecho", repeticiones: "10x3" },
                Ejercicio { nombre: "Banda hacia abajo con brazos rectos", repeticiones: "10x3" },
                Ejercicio { nombre: "Banda hacia abajo (brazo en ángulo recto)", repeticiones: "10x3" },
                Ejercicio { nombre: "Bandita ángulo recto int. (por brazo)", repeticiones: "10x3" },
                Ejercicio { nombre: "Bandita ángulo recto ext. (por brazo)", repeticiones: "10x3" },
            ],
        },
        Categoria {
            titulo: "Pared y Rodillo",
            ejercicios: vec![
                Ejercicio { nombre: "Pegado a pared brazos en T", repeticiones: "10x3" },
                Ejercicio { nombre: "Estiramiento rodillo brazos en pared", repeticiones: "10x3" },
                Ejercicio { nombre: "Estiramiento rodillo lateral", repeticiones: "10x3" },
            ],
        },
        Categoria {
            titulo: "TRX",
            ejercicios: vec![
                Ejercicio { nombre: "Codos pegados al cuerpo, hombros relajados", repeticiones: "10x3" },
                Ejercicio { nombre: "Codos separados (T), hombros relajados", repeticiones: "10x3" },
            ],
        },
    ];

    let mut check_buttons: Vec<CheckButton> = Vec::new();
    let mut total_ejercicios = 0;

    for categoria in rutina {
        let group = PreferencesGroup::builder().title(categoria.titulo).build();

        for ej in categoria.ejercicios {
            let check = CheckButton::builder().valign(Align::Center).build();
            check_buttons.push(check.clone());
            total_ejercicios += 1;

            let row = ActionRow::builder()
                .title(ej.nombre)
                .subtitle(ej.repeticiones)
                .activatable_widget(&check)
                .build();
            
            row.add_prefix(&check);
            group.add(&row);
        }
        content_box.append(&group);
    }

    let separator = Separator::builder().orientation(Orientation::Horizontal).margin_top(12).margin_bottom(12).build();
    content_box.append(&separator);

    let eval_group = PreferencesGroup::builder().title("Evaluación Post-Sesión").build();
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
    content_box.append(&eval_group);

    let hoy_wrapper = Box::builder().orientation(Orientation::Vertical).build();
    hoy_wrapper.append(&banner);
    
    let scrolled_hoy = ScrolledWindow::builder().child(&content_box).vexpand(true).build();
    hoy_wrapper.append(&scrolled_hoy);

    // VISTA "PROGRESO" (Heatmap y Estadísticas)
    let progress_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(18)
        .margin_top(24)
        .margin_start(24)
        .margin_end(24)
        .build();

    let total_sesiones = history.len();
    let stats_title = Label::builder()
        .label("<b>Resumen de Rehabilitación</b>")
        .use_markup(true)
        .halign(Align::Start)
        .build();

    let stats_label = Label::builder()
        .label(&format!("Sesiones totales registradas: <b>{}</b>", total_sesiones))
        .use_markup(true)
        .halign(Align::Start)
        .build();

    progress_box.append(&stats_title);
    progress_box.append(&stats_label);

    // --- LÓGICA DEL HEATMAP ---
    let heatmap_title = Label::builder()
        .label("<b>Adherencia Mensual (Últimas 5 Semanas)</b>")
        .use_markup(true)
        .halign(Align::Start)
        .margin_top(24)
        .build();
    progress_box.append(&heatmap_title);

    let heatmap_grid = Grid::builder()
        .halign(Align::Center)
        .row_spacing(4)
        .column_spacing(4)
        .margin_top(12)
        .build();

    let hoy = Local::now().date_naive();
    let dias_a_mostrar = 35; // 5 semanas
    let fecha_inicio = hoy - Duration::days(dias_a_mostrar - 1);

    for i in 0..dias_a_mostrar {
        let fecha_actual = fecha_inicio + Duration::days(i);
        let fecha_str = fecha_actual.format("%Y-%m-%d").to_string();

        let mut clase_intensidad = "heatmap-0";
        let mut tooltip = format!("{} - Sin registro", fecha_str);

        // Buscar si hubo sesión ese día
        if let Some(sesion) = history.iter().find(|r| r.fecha.starts_with(&fecha_str)) {
            let porcentaje = sesion.ejercicios_completados as f64 / sesion.ejercicios_totales as f64;
            if porcentaje > 0.8 {
                clase_intensidad = "heatmap-4";
            } else if porcentaje > 0.5 {
                clase_intensidad = "heatmap-3";
            } else if porcentaje > 0.2 {
                clase_intensidad = "heatmap-2";
            } else {
                clase_intensidad = "heatmap-1";
            }
            tooltip = format!("{} - {}% completado (Molestia: {})", fecha_str, (porcentaje * 100.0) as i32, sesion.nivel_molestia);
        }

        let cell = Box::builder()
            .css_classes(["heatmap-box", clase_intensidad])
            .tooltip_text(&tooltip)
            .build();

        // 5 filas (semanas) x 7 columnas (días)
        let columna = (i % 7) as i32;
        let fila = (i / 7) as i32;
        heatmap_grid.attach(&cell, columna, fila, 1, 1);
    }
    
    progress_box.append(&heatmap_grid);

    let scrolled_progreso = ScrolledWindow::builder().child(&progress_box).vexpand(true).build();

    let view_stack = ViewStack::builder().build();
    view_stack.add_titled_with_icon(&hoy_wrapper, Some("hoy"), "Hoy", "view-list-symbolic");
    view_stack.add_titled_with_icon(&scrolled_progreso, Some("progreso"), "Progreso", "office-chart-line-symbolic");

    let view_switcher = ViewSwitcher::builder()
        .stack(&view_stack)
        .policy(adw::ViewSwitcherPolicy::Narrow)
        .build();

    let header_bar = HeaderBar::builder()
        .title_widget(&view_switcher)
        .build();

    let toolbar_view = ToolbarView::builder().build();
    toolbar_view.add_top_bar(&header_bar);
    toolbar_view.set_content(Some(&view_stack));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Bosu")
        .default_width(450)
        .default_height(750)
        .content(&toolbar_view)
        .build();

    window.present();
}
