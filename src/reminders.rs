use crate::storage::Repository;
use adw::prelude::*;
use chrono::Timelike;
pub fn install(app: &adw::Application, repo: Repository) {
    let snooze = gtk::gio::SimpleAction::new("snooze", None);
    snooze.connect_activate(gtk::glib::clone!(
        #[strong]
        repo,
        move |_, _| {
            let tomorrow = (chrono::Local::now().date_naive() + chrono::Duration::days(1))
                .format("%Y-%m-%d")
                .to_string();
            if let Err(e) = repo.update(|d| {
                d.preferences.snooze_until = tomorrow;
                Ok(())
            }) {
                eprintln!("No se pudo posponer el recordatorio: {e}");
            }
        }
    ));
    app.add_action(&snooze);
    let weak = app.downgrade();
    let tick = move || {
        let Some(app) = weak.upgrade() else {
            return gtk::glib::ControlFlow::Break;
        };
        let data = repo.snapshot();
        let now = chrono::Local::now();
        let today = now.format("%Y-%m-%d").to_string();
        let p = &data.preferences;
        if p.reminders
            && p.reminder_last_date != today
            && (p.snooze_until.is_empty() || today >= p.snooze_until)
            && now.hour() * 60 + now.minute() >= p.reminder_hour * 60 + p.reminder_minute
            && !data.sessions.iter().any(|s| s.fecha == today)
        {
            let notification = gtk::gio::Notification::new(&crate::i18n::tr("Un momento para ti"));
            notification.set_body(Some(&crate::i18n::tr(
                "¿Te viene bien registrar cómo estás? Puedes practicar, adaptar o descansar.",
            )));
            notification.set_default_action("app.open-today");
            notification.add_button(&crate::i18n::tr("Más tarde"), "app.snooze");
            if repo
                .update(|d| {
                    d.preferences.reminder_last_date = today;
                    Ok(())
                })
                .is_ok()
            {
                app.send_notification(Some("bosu-gentle-reminder"), &notification);
            }
        }
        gtk::glib::ControlFlow::Continue
    };
    gtk::glib::timeout_add_seconds_local(60, tick);
}
