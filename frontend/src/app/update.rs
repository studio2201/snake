//! `App` lifecycle and message dispatch.
//!
//! The [`App::create_app`] constructor seeds the initial state and fires the
//! startup probes; [`App::update_app`] is a one-line dispatcher that hands
//! each [`Msg`] variant off to the focused handler in
//! [`crate::app::handlers`].

use crate::api::StorageService;
use crate::app::{App, Msg};
use yew::prelude::*;

impl App {
    /// Builds the initial [`App`] state and kicks off the two startup
    /// probes (`/api/config` and `/api/pin-required`).
    pub fn create_app(ctx: &Context<Self>) -> Self {
        let theme = StorageService::get_theme();
        let locale_state = crate::i18n::get_saved_locale();

        // Apply the persisted theme to the <html> element so the CSS
        // variables resolve before the first paint.
        if let Some(win) = web_sys::window()
            && let Some(doc) = win.document()
            && let Some(el) = doc.document_element()
        {
            let _ = el.set_attribute("data-theme", &theme);
            let _ = el.set_attribute("class", &theme);
        }

        App::spawn_startup_probes(ctx);

        Self {
            // Defaults: optimistic "unauthenticated" with empty strings so
            // the header doesn't flash a stale version/title before the
            // /api/config response arrives.
            authenticated: false,
            app_version: String::new(),
            site_title: String::new(),
            theme,
            locale_state,
            active_notification: None,
            // Treat PIN as required until the backend disagrees; this avoids
            // briefly exposing the game UI to unauthenticated users.
            is_pin_required: true,
            // Defaults align with backend `config.rs::assemble`: translation
            // is on by default, themes on, print off. Once `/api/config`
            // resolves, the backend value wins.
            enable_translation: true,
            enable_themes: true,
            enable_print: false,
            show_version: true,
            show_github: true,
        }
    }

    /// Routes each [`Msg`] variant to its dedicated handler.
    ///
    /// The body of this function is intentionally trivial: every branch
    /// returns the `bool` from the handler so the framework knows whether
    /// a re-render is required.
    pub fn update_app(&mut self, ctx: &Context<Self>, msg: Msg) -> bool {
        match msg {
            Msg::LoadConfig(config) => self.handle_load_config(ctx, config),
            Msg::LoadPinRequired(req) => self.handle_load_pin_required(ctx, req),
            Msg::SetAuthenticated(auth) => self.handle_set_authenticated(ctx, auth),
            Msg::SwitchLanguage(lang) => self.handle_switch_language(ctx, lang),
            Msg::ToggleTheme => self.handle_toggle_theme(ctx),
            Msg::Logout => self.handle_logout(ctx),
            Msg::SetStatus(status) => self.handle_set_status(ctx, status),
            Msg::OnlineStatusChanged(online) => self.handle_online_status_changed(ctx, online),
            Msg::Print => self.handle_print(ctx),
        }
    }
}
