//! Run on a real display or under Xvfb; no personal data is used.
use super::*;
fn children(widget: &impl IsA<gtk::Widget>) -> Vec<gtk::Widget> {
    let mut out = vec![widget.clone().upcast::<gtk::Widget>()];
    let mut child = widget.first_child();
    while let Some(current) = child {
        out.extend(children(&current));
        child = current.next_sibling();
    }
    out
}
fn buttons(window: &impl IsA<gtk::Widget>, label: &str) -> Vec<gtk::Button> {
    children(window)
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Button>().ok())
        .filter(|b| b.label().as_deref() == Some(label))
        .collect()
}
fn dropdown(window: &impl IsA<gtk::Widget>, first: &str) -> gtk::DropDown {
    children(window)
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::DropDown>().ok())
        .find(|d| {
            d.model()
                .and_downcast::<gtk::StringList>()
                .and_then(|m| m.string(0))
                .as_deref()
                == Some(first)
        })
        .expect(first)
}
fn pump() {
    let context = glib::MainContext::default();
    for _ in 0..100 {
        if !context.pending() {
            break;
        }
        context.iteration(false);
    }
}
fn capture(window: &adw::ApplicationWindow, name: &str) {
    let Ok(directory) = std::env::var("BOSU_SCREENSHOT_DIR") else {
        return;
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(180);
    while std::time::Instant::now() < deadline {
        pump();
        std::thread::sleep(std::time::Duration::from_millis(3));
    }
    let path = std::path::Path::new(&directory);
    std::fs::create_dir_all(path).unwrap();
    let paintable = gtk::WidgetPaintable::new(Some(window));
    let snapshot = gtk::Snapshot::new();
    paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);
    let node = snapshot.to_node().expect("allocated widget snapshot");
    window
        .renderer()
        .unwrap()
        .render_texture(&node, None)
        .save_to_png(path.join(name))
        .unwrap();
}
#[test]
#[ignore = "requires a desktop or Xvfb; CI runs this test explicitly"]
fn ui_smoke_partial_session_linked_map_navigation_and_draft() {
    adw::init().unwrap();
    i18n::init("es");
    gio::resources_register_include!("bosu.gresource").unwrap();
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("style.css"));
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    let app = adw::Application::builder()
        .application_id("org.diego.Bosu.Test")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.register(gio::Cancellable::NONE).unwrap();
    let root = std::env::temp_dir().join(format!("bosu-ui-{}", glib::uuid_string_random()));
    let repo = storage::Repository::open(root.clone()).unwrap();
    let window = adw::ApplicationWindow::builder()
        .application(&app)
        .default_width(860)
        .default_height(900)
        .build();
    install_close_guard(&window);
    rebuild(&window, &repo);
    window.present();
    pump();
    let routine = dropdown(&window, "Descanso o sensaciones");
    routine.set_selected(1);
    pump();
    dropdown(&window, "Pendiente").set_selected(1);
    pump();
    capture(&window, "today.png");
    buttons(&window, "Registrar")[0].emit_clicked();
    pump();
    let map = gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Window>().ok())
        .find(|w| w.title().as_deref() == Some("Cómo estoy · antes"))
        .unwrap();
    dropdown(&map, "Seleccionar zona por nombre…").set_selected(1);
    buttons(&map, "Marcar zona seleccionada")[0].emit_clicked();
    pump();
    map.close();
    pump();
    assert!(
        repo.snapshot()
            .drafts
            .values()
            .any(|d| d.mapa_antes.as_ref().is_some_and(|m| m.points.len() == 1))
    );
    buttons(&window, "Guardar mi avance")[0].emit_clicked();
    pump();
    let data = repo.snapshot();
    assert_eq!(data.sessions.len(), 1);
    assert_eq!(data.sessions[0].estado, models::SessionState::Partial);
    assert_eq!(data.sessions[0].ejercicios_completados, 1);
    assert_eq!(data.sessions[0].dolor, None);
    assert_eq!(repo.all_maps().len(), 1);
    app.lookup_action("open-body").unwrap().activate(None);
    pump();
    capture(&window, "body.png");
    app.lookup_action("open-progress").unwrap().activate(None);
    pump();
    capture(&window, "progress.png");
    app.lookup_action("open-today").unwrap().activate(None);
    pump();
    routine.set_selected(0);
    pump();
    let notes = children(&window)
        .into_iter()
        .filter_map(|w| w.downcast::<adw::EntryRow>().ok())
        .find(|r| r.title() == "Notas de la sesión · opcional")
        .unwrap();
    notes.set_text("Descanso <suave> & sin presión");
    pump();
    window.close();
    pump();
    assert_eq!(
        repo.snapshot().drafts["__rest__"].notas,
        "Descanso <suave> & sin presión"
    );
    assert_eq!(
        storage::Repository::open(root.clone())
            .unwrap()
            .snapshot()
            .sessions
            .len(),
        1
    );
    std::fs::remove_dir_all(root).unwrap();
}
