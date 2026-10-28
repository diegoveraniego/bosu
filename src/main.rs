use adw::prelude::*;
use gtk::{gio, glib};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
mod appearance;
mod focus_mode;
mod focus_title;
mod i18n;
mod models;
mod motion;
pub use models::{Categoria, Configuracion, Ejercicio, RegistroSesion, Rutina};
mod body_evolution;
mod body_gif;
mod body_heat;
mod body_legacy;
mod body_map;
mod body_regions;
mod kine;
mod progress_page;
mod reminders;
mod session_page;
mod settings_editor;
mod storage;
#[cfg(test)]
mod ui_tests;
mod weekly_goal;
type Flush = Rc<dyn Fn() -> Result<(), String>>;
const APP_ID: &str = "org.diego.Bosu";
fn toast(message: &str) -> adw::Toast {
    adw::Toast::builder()
        .title(message)
        .use_markup(false)
        .build()
}
fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.add_main_option(
        "version",
        glib::Char::from(b'v'),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        "Print version",
        None,
    );
    app.connect_handle_local_options(|_, options| {
        if options.contains("version") {
            println!("Bosu {}", env!("CARGO_PKG_VERSION"));
            std::ops::ControlFlow::Break(glib::ExitCode::SUCCESS)
        } else {
            std::ops::ControlFlow::Continue(())
        }
    });
    app.connect_startup(|app| {
        i18n::init("");
        gio::resources_register_include!("bosu.gresource").expect("Bosu embedded resources");
        let provider = gtk::CssProvider::new();
        provider.load_from_string(include_str!("style.css"));
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
            let icons = gtk::IconTheme::for_display(&display);
            icons.add_resource_path("/org/diego/Bosu/icons");
        }
        for (action, keys) in [
            ("app.preferences", vec!["<Control>comma"]),
            ("app.quit", vec!["<Control>q"]),
            ("app.open-today", vec!["<Control>1"]),
            ("app.open-body", vec!["<Control>3"]),
            ("app.open-progress", vec!["<Control>2"]),
            ("app.shortcuts", vec!["<Control>question"]),
        ] {
            app.set_accels_for_action(action, &keys);
        }
    });
    app.connect_activate(|app| {
        if let Some(window) = app.active_window() {
            window.present();
            return;
        }
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title(crate::i18n::tr("Bosu"))
            .icon_name(APP_ID)
            .default_width(450)
            .default_height(800)
            .build();
        install_close_guard(&window);
        match storage::Repository::open(glib::user_data_dir()) {
            Ok(repo) => {
                i18n::init(&repo.snapshot().preferences.language);
                rebuild(&window, &repo);
                reminders::install(app, repo);
            }
            Err(error) => recovery(&window, error),
        }
        window.present();
        // Launch into the guided session. Rebuilding the journal after closing
        // it must not open the guided window again.
        if let Some(focus) = app.lookup_action("focus-preview") {
            focus.activate(None);
        }
    });
    app.run()
}
fn show_active_dialog(window: &adw::ApplicationWindow) -> bool {
    for widget in gtk::Window::list_toplevels() {
        if let Ok(child) = widget.downcast::<gtk::Window>()
            && child.is_visible()
            && ((child.is_modal() && child.transient_for().as_ref() == Some(window.upcast_ref()))
                || (child.widget_name() == "focus-session"
                    && child.application() == window.application()))
        {
            child.present();
            return true;
        }
    }
    false
}
fn rebuild(window: &adw::ApplicationWindow, repo: &storage::Repository) {
    appearance::apply(&repo.snapshot().preferences.appearance);
    let toasts = adw::ToastOverlay::new();
    let stack = adw::ViewStack::new();
    let progress = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .margin_top(24)
        .margin_bottom(32)
        .margin_start(16)
        .margin_end(16)
        .build();
    progress_page::populate(&progress, repo);
    let body_host = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let dirty_body = Rc::new(Cell::new(false));
    let changed: Rc<dyn Fn()> = Rc::new(glib::clone!(
        #[weak]
        progress,
        #[strong]
        repo,
        #[strong]
        dirty_body,
        move || {
            progress_page::populate(&progress, &repo);
            dirty_body.set(true);
        }
    ));
    let session = session_page::build(repo.clone(), changed.clone(), toasts.clone());
    let body = body_map::build(repo.clone(), changed.clone());
    body_host.append(&body.widget);
    let body_flush = Rc::new(RefCell::new(body.flush));
    let session_flush = session.flush;
    let flush: Rc<dyn Fn() -> Result<(), String>> = Rc::new(glib::clone!(
        #[strong]
        session_flush,
        #[strong]
        body_flush,
        move || {
            session_flush()?;
            (body_flush.borrow())()
        }
    ));
    let clamp = adw::Clamp::builder()
        .maximum_size(720)
        .tightening_threshold(520)
        .child(&session.widget)
        .build();
    stack.add_titled_with_icon(
        &gtk::ScrolledWindow::builder()
            .child(&clamp)
            .vexpand(true)
            .build(),
        Some("hoy"),
        &i18n::tr("Hoy"),
        "view-list-symbolic",
    );
    let clamp = adw::Clamp::builder()
        .maximum_size(720)
        .child(&progress)
        .build();
    stack.add_titled_with_icon(
        &gtk::ScrolledWindow::builder()
            .child(&clamp)
            .vexpand(true)
            .build(),
        Some("progreso"),
        &i18n::tr("Progreso"),
        "charge-symbolic",
    );
    stack.add_titled_with_icon(
        &body_host,
        Some("cuerpo"),
        &i18n::tr("Cuerpo"),
        "avatar-default-symbolic",
    );
    let guard = Rc::new(Cell::new(false));
    let previous = Rc::new(RefCell::new("hoy".to_string()));
    stack.connect_visible_child_name_notify(glib::clone!(
        #[strong]
        flush,
        #[strong]
        repo,
        #[strong]
        changed,
        #[strong]
        dirty_body,
        #[strong]
        body_flush,
        #[weak]
        body_host,
        #[weak]
        toasts,
        move |stack| {
            if guard.get() {
                return;
            }
            if let Err(error) = flush() {
                toasts.add_toast(crate::toast(&crate::i18n::tr(&format!(
                    "No se pudo guardar el borrador: {error}"
                ))));
                guard.set(true);
                stack.set_visible_child_name(&previous.borrow());
                guard.set(false);
                return;
            }
            let name = stack
                .visible_child_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "hoy".into());
            if name == "cuerpo" && dirty_body.replace(false) {
                while let Some(child) = body_host.first_child() {
                    body_host.remove(&child);
                }
                let panel = body_map::build(repo.clone(), changed.clone());
                *body_flush.borrow_mut() = panel.flush;
                body_host.append(&panel.widget);
            }
            *previous.borrow_mut() = name;
        }
    ));
    let switcher = adw::ViewSwitcher::builder()
        .stack(&stack)
        .policy(adw::ViewSwitcherPolicy::Narrow)
        .build();
    let header = adw::HeaderBar::builder().title_widget(&switcher).build();
    header.pack_end(
        &gtk::Button::builder()
            .icon_name("emblem-system-symbolic")
            .action_name("app.preferences")
            .tooltip_text(i18n::tr("Preferencias"))
            .build(),
    );
    header.pack_start(
        &gtk::Button::builder()
            .icon_name("help-about-symbolic")
            .action_name("app.about")
            .tooltip_text(i18n::tr("Acerca de Bosu"))
            .build(),
    );
    let focus = gio::SimpleAction::new("focus-preview", None);
    focus.connect_activate(glib::clone!(
        #[weak]
        window,
        #[strong]
        repo,
        #[strong]
        flush,
        #[weak]
        toasts,
        move |_, _| {
            if let Err(error) = flush() {
                toasts.add_toast(toast(&error));
                return;
            }
            if show_active_dialog(&window) {
                return;
            }
            let guided = focus_mode::present(&window, &repo);
            guided.connect_hide(glib::clone!(
                #[weak]
                window,
                #[strong]
                repo,
                move |_| {
                    rebuild(&window, &repo);
                    window.present();
                }
            ));
        }
    ));
    if let Some(app) = window.application() {
        app.add_action(&focus);
    }
    let testing = gio::SimpleAction::new("motion-testing", None);
    testing.connect_activate(glib::clone!(
        #[weak]
        toasts,
        move |_, _| {
            if let Err(error) = motion::open() {
                toasts.add_toast(toast(&format!("No se pudo abrir Testing: {error}")));
            }
        }
    ));
    if let Some(app) = window.application() {
        app.add_action(&testing);
    }
    let menu = gio::Menu::new();
    menu.append(Some("Modo enfoque (prueba)"), Some("app.focus-preview"));
    menu.append(
        Some("Testing · Movimiento (experimental)"),
        Some("app.motion-testing"),
    );
    menu.append(Some(&i18n::tr("Preferencias")), Some("app.preferences"));
    menu.append(Some(&i18n::tr("Atajos de teclado")), Some("app.shortcuts"));
    menu.append(Some(&i18n::tr("Acerca de Bosu")), Some("app.about"));
    header.pack_end(
        &gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&menu)
            .tooltip_text(crate::i18n::tr("Menú principal"))
            .build(),
    );
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&stack));
    toasts.set_child(Some(&toolbar));
    window.set_content(Some(&toasts));
    // Replace the previous close handler instead of retaining stale editor callbacks.
    let app = window.application().unwrap();
    for (name, page) in [
        ("open-today", "hoy"),
        ("open-body", "cuerpo"),
        ("open-progress", "progreso"),
    ] {
        let action = gio::SimpleAction::new(name, None);
        action.connect_activate(glib::clone!(
            #[weak]
            stack,
            #[weak]
            window,
            move |_, _| {
                if show_active_dialog(&window) {
                    return;
                }
                stack.set_visible_child_name(page);
                window.present();
            }
        ));
        app.add_action(&action);
    }
    let preferences = gio::SimpleAction::new("preferences", None);
    preferences.connect_activate(glib::clone!(
        #[weak]
        window,
        #[weak]
        toasts,
        #[strong]
        repo,
        #[strong]
        flush,
        move |_, _| {
            if show_active_dialog(&window) {
                return;
            }
            if let Err(e) = flush() {
                toasts.add_toast(crate::toast(&crate::i18n::tr(&format!(
                    "No se pudo guardar el borrador: {e}"
                ))));
                return;
            }
            let on_save: Rc<dyn Fn()> = Rc::new(glib::clone!(
                #[weak]
                window,
                #[strong]
                repo,
                move || rebuild(&window, &repo)
            ));
            settings_editor::show_settings_window(&window, repo.clone(), on_save, toasts);
        }
    ));
    app.add_action(&preferences);
    let about = gio::SimpleAction::new("about", None);
    about.connect_activate(glib::clone!(
        #[weak]
        window,
        move |_, _| {
            let about = adw::AboutWindow::builder()
                .transient_for(&window)
                .modal(true)
                .application_name("Bosu")
                .application_icon(APP_ID)
                .developer_name("diegoveraniego")
                .version(env!("CARGO_PKG_VERSION"))
                .comments(crate::i18n::tr(
                    "Registra cómo responde tu cuerpo, a tu ritmo.\n\nPuedes guardar sesiones completas, parciales, sensaciones o descanso. Las sesiones parciales cuentan como días de práctica.\n\nLos borradores se guardan en este equipo. Los mapas antes y después quedan vinculados a cada sesión. Dolor y esfuerzo son mediciones independientes y opcionales.\n\nUna zona sin marcar significa sin datos, no dolor cero. Los promedios solo usan mediciones explícitas. Los datos antiguos se conservan sin reinterpretarlos.\n\nLos recordatorios son opcionales y posponibles. Tus datos son locales; puedes respaldarlos, restaurarlos y deshacer la última eliminación.",
                ))
                .website("https://github.com/diegoveraniego/bosu")
                .issue_url("https://github.com/diegoveraniego/bosu/issues")
                .license_type(gtk::License::MitX11)
                .build();
            about.add_credit_section(
                Some("Ilustración corporal"),
                &["hugobonifay/body-map · ver atribución incluida"],
            );
            about.present();
        }
    ));
    app.add_action(&about);
    let shortcuts = gio::SimpleAction::new("shortcuts", None);
    shortcuts.connect_activate(glib::clone!(
        #[weak]
        window,
        move |_, _| show_shortcuts(&window)
    ));
    app.add_action(&shortcuts);
    let quit = gio::SimpleAction::new("quit", None);
    quit.connect_activate(glib::clone!(
        #[weak]
        window,
        move |_, _| window.close()
    ));
    app.add_action(&quit);
    // Store one current close guard in a window-owned action; the signal itself is installed once.
    let close = gio::SimpleAction::new("flush", None);
    close.connect_activate(glib::clone!(
        #[strong]
        flush,
        #[weak]
        toasts,
        move |_, _| {
            if let Err(e) = flush() {
                toasts.add_toast(crate::toast(&crate::i18n::tr(&e)));
            }
        }
    ));
    window.add_action(&close);
    CLOSE_GUARDS.with(|guards| {
        guards.borrow_mut().insert(window.as_ptr() as usize, flush);
    });
}
thread_local! {static CLOSE_GUARDS:RefCell<std::collections::HashMap<usize,Flush>>=RefCell::new(std::collections::HashMap::new());}
fn install_close_guard(window: &adw::ApplicationWindow) {
    window.connect_close_request(|window| {
        // Flush transient map editors before the parent session draft.
        for widget in gtk::Window::list_toplevels() {
            if let Ok(child) = widget.downcast::<gtk::Window>()
                && child.is_visible()
                && child.transient_for().as_ref() == Some(window.upcast_ref())
            {
                child.close();
                if child.is_visible() {
                    return glib::Propagation::Stop;
                }
            }
        }
        let key = window.as_ptr() as usize;
        let result = CLOSE_GUARDS.with(|g| g.borrow().get(&key).map_or(Ok(()), |flush| flush()));
        if let Err(e) = result {
            let dialog = adw::MessageDialog::new(
                Some(window),
                Some(&i18n::tr("El borrador todavía no está guardado")),
                Some(&format!(
                    "Tus cambios siguen abiertos. Puedes reintentar guardar antes de cerrar.\n{e}"
                )),
            );
            dialog.add_response("continue", &i18n::tr("Seguir editando"));
            dialog.add_response(
                "discard",
                &i18n::tr("Salir sin guardar los últimos cambios"),
            );
            dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
            dialog.set_default_response(Some("continue"));
            dialog.set_close_response("continue");
            dialog.connect_response(
                Some("discard"),
                glib::clone!(
                    #[weak]
                    window,
                    move |_, _| {
                        CLOSE_GUARDS.with(|guards| {
                            guards.borrow_mut().remove(&(window.as_ptr() as usize));
                        });
                        window.close();
                    }
                ),
            );
            dialog.present();
            glib::Propagation::Stop
        } else {
            CLOSE_GUARDS.with(|g| {
                g.borrow_mut().remove(&key);
            });
            glib::Propagation::Proceed
        }
    });
}
fn show_shortcuts(parent: &adw::ApplicationWindow) {
    let group = adw::PreferencesGroup::builder()
        .title(crate::i18n::tr("Atajos de teclado"))
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();
    for (title, keys) in [
        ("Hoy", "<Control>1"),
        ("Cuerpo", "<Control>3"),
        ("Progreso", "<Control>2"),
        ("Preferencias", "<Control>comma"),
        ("Cerrar Bosu", "<Control>q"),
    ] {
        let row = adw::ActionRow::builder().title(title).build();
        row.add_suffix(&gtk::ShortcutLabel::builder().accelerator(keys).build());
        group.add(&row);
    }
    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    view.set_content(Some(&group));
    adw::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(crate::i18n::tr("Atajos de teclado"))
        .content(&view)
        .build()
        .present();
}
fn recovery(window: &adw::ApplicationWindow, error: String) {
    let status = adw::StatusPage::builder()
        .icon_name("dialog-error-symbolic")
        .title(crate::i18n::tr("Tus datos necesitan revisión"))
        .description(crate::i18n::tr(&format!(
            "No se reemplazó ningún archivo.\n{error}"
        )))
        .build();
    let actions = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .halign(gtk::Align::Center)
        .build();
    let retry = gtk::Button::with_label(&crate::i18n::tr("Reintentar"));
    let folder = gtk::Button::with_label(&crate::i18n::tr("Abrir carpeta de datos"));
    actions.append(&retry);
    actions.append(&folder);
    let restore = gtk::Button::with_label(&i18n::tr("Restaurar respaldo…"));
    actions.append(&restore);
    restore.connect_clicked(glib::clone!(#[weak] window,#[weak] status,move |_| {
        let dialog=gtk::FileDialog::builder().title(i18n::tr("Elegir respaldo de Bosu")).build();let window=window.clone();let status=status.clone();
        dialog.open(Some(&window.clone()),gio::Cancellable::NONE,move |result| {
            let Ok(file)=result else{return;};let Some(path)=file.path()else{return;};
            match storage::Repository::inspect_backup(&path) {
                Err(error)=>status.set_description(Some(&error)),
                Ok(_)=>{
                    let confirm=adw::MessageDialog::new(Some(&window),Some(&i18n::tr("¿Restaurar este respaldo?")),Some(&i18n::tr("Se conservará una copia de los archivos actuales antes de restaurar los datos.")));
                    confirm.add_responses(&[("cancel",&i18n::tr("Cancelar")),("restore",&i18n::tr("Restaurar"))]);confirm.set_default_response(Some("cancel"));confirm.set_close_response("cancel");
                    confirm.connect_response(Some("restore"),move|_,_|match storage::Repository::recover(glib::user_data_dir(),&path){Ok(repo)=>{i18n::init(&repo.snapshot().preferences.language);rebuild(&window,&repo);if let Some(app)=window.application().and_downcast::<adw::Application>(){reminders::install(&app,repo);}},Err(error)=>status.set_description(Some(&error))});confirm.present();
                }
            }
        });
    }));
    status.set_child(Some(&actions));
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&status));
    window.set_content(Some(&toolbar));
    retry.connect_clicked(glib::clone!(
        #[weak]
        window,
        #[weak]
        status,
        move |_| match storage::Repository::open(glib::user_data_dir()) {
            Ok(repo) => {
                rebuild(&window, &repo);
                if let Some(app) = window.application().and_downcast::<adw::Application>() {
                    reminders::install(&app, repo);
                }
            }
            Err(e) => status.set_description(Some(&e)),
        }
    ));
    folder.connect_clicked(glib::clone!(
        #[weak]
        status,
        move |_| {
            let file = gio::File::for_path(glib::user_data_dir());
            if let Err(e) = gio::AppInfo::launch_default_for_uri(
                file.uri().as_str(),
                None::<&gio::AppLaunchContext>,
            ) {
                status.set_description(Some(&e.to_string()));
            }
        }
    ));
}
