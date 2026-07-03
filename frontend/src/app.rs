//! Root application component and message model.
//!
//! [`App`] is the single top-level Yew component mounted by [`crate::main`].
//! It owns the global state machine (theme, locale, online status, auth) and
//! drives all other components via props and the [`LocaleContext`]
//! (re-exported from [`crate::i18n`]).
//!
//! Message handling is split into focused `impl App` methods defined in
//! [`update`]; the `view` lives in [`view`]. Submodules are kept small (≤250
//! lines each) to preserve compile throughput on the WASM target.

pub mod handlers;
pub mod update;
pub mod view;

use crate::api::ConfigResponse;
use std::cell::RefCell;
use yew::prelude::*;

// Live mirror of the latest `/api/config` version, shared between the
// `update` override (writer) and the SW `message` listener closure
// (reader). Held in a thread-local because the closure outlives the
// component instance and the Yew `App` struct is fixed-shape — adding
// a field here would require touching `app::update::create_app` which
// is outside this file's allowed edits. In a single-threaded WASM
// runtime this behaves like a process-wide static.
thread_local! {
    static APP_VERSION: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Messages dispatched by child components and DOM event listeners.
///
/// Each variant corresponds to a discrete user-visible transition or
/// network-driven update; the handler methods on `App` (in [`update`])
/// keep each arm small enough to stay under the
/// [`clippy::cognitive-complexity-threshold`](../../../../clippy.toml).
pub enum Msg {
    /// Backend configuration arrived from [`crate::api::ApiService::get_config`].
    LoadConfig(ConfigResponse),
    /// Backend answered whether a PIN gate is required.
    LoadPinRequired(bool),
    /// Auth state changed (login success, logout, programmatic unlock).
    SetAuthenticated(bool),
    /// User picked a new locale in the header language picker.
    SwitchLanguage(String),
    /// User clicked the theme toggle.
    ToggleTheme,
    /// User clicked the logout button.
    Logout,
    /// Footer status banner text and severity level.
    SetStatus(Option<(String, String)>),
    /// Browser fired `online` / `offline` events.
    OnlineStatusChanged(bool),
    /// User clicked the print button.
    Print,
}

/// Top-level application state.
///
/// All fields are populated by [`update::App::create_app`] and the message
/// handlers defined in [`update`]. The defaults are conservative: the
/// `app_version` and `site_title` are empty until the `/api/config` response
/// arrives so the UI never flashes a stale string.
pub struct App {
    /// `true` once the user has cleared the PIN gate (or none was required).
    pub authenticated: bool,
    /// Backend version string; `""` until [`Msg::LoadConfig`] resolves.
    pub app_version: String,
    /// Site title shown in the header; `""` until [`Msg::LoadConfig`] resolves.
    pub site_title: String,
    /// Canonical theme name (e.g. `"brinstar"`, `"tourian"`).
    pub theme: String,
    /// Current locale code (`"en"`, `"de"`, ...).
    pub locale_state: String,
    /// Optional `(message, css_class)` for the footer's transient status banner.
    pub active_notification: Option<(String, String)>,
    /// Whether the PIN gate is required to access the game.
    pub is_pin_required: bool,
    /// Show the language picker in the header.
    pub enable_translation: bool,
    /// Show the theme toggle in the header.
    pub enable_themes: bool,
    /// Show the print button in the header.
    pub enable_print: bool,
    /// Show the version badge in the header.
    pub show_version: bool,
    /// Show the GitHub link in the footer.
    pub show_github: bool,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        Self::create_app(ctx)
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        // Mirror the freshest `/api/config` version into the shared cell
        // before the handler runs, so the SW message listener always
        // reads the latest value. The real config handling still runs
        // via `update_app` below; this only touches the side channel.
        if let Msg::LoadConfig(config) = &msg {
            APP_VERSION.with(|v| *v.borrow_mut() = config.version.clone());
        }
        self.update_app(ctx, msg)
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        self.view_app(ctx)
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            use wasm_bindgen::JsCast;
            // The renderer only runs in the browser, so `window()` is safe to
            // unwrap. Documented per the "no unwrap in non-test code" rule.
            let window = web_sys::window().expect("renderer runs in a browser window");

            let link_online = ctx.link().clone();
            let on_online =
                wasm_bindgen::prelude::Closure::<dyn FnMut(_)>::new(move |_: web_sys::Event| {
                    link_online.send_message(Msg::OnlineStatusChanged(true));
                });
            window
                .add_event_listener_with_callback("online", on_online.as_ref().unchecked_ref())
                .expect("failed to register online listener");
            on_online.forget();

            let link_offline = ctx.link().clone();
            let on_offline =
                wasm_bindgen::prelude::Closure::<dyn FnMut(_)>::new(move |_: web_sys::Event| {
                    link_offline.send_message(Msg::OnlineStatusChanged(false));
                });
            window
                .add_event_listener_with_callback("offline", on_offline.as_ref().unchecked_ref())
                .expect("failed to register offline listener");
            on_offline.forget();

            // Service-worker update handshake.
            //
            // We subscribe to BOTH `message` (the SW's explicit
            // `CACHE_UPDATED` ping after a new version installs) and
            // `controllerchange` (fires when `clients.claim()` takes
            // over this page). The two paths overlap but are not
            // identical: `message` can be missed if the SW activated
            // while the page was in bfcache or before our listener was
            // bound; `controllerchange` fires even when no `postMessage`
            // ever reached this client (e.g. a fresh tab opened against
            // an already-upgraded cache). Subscribing to both keeps the
            // handshake reliable across realistic post-deployment flows.
            // The defensive version compare on `message` prevents a
            // self-reload when the SW is announcing a version we already
            // mirror from `/api/config`.
            //
            // `navigator.serviceWorker` and `addEventListener` are
            // reached via `js_sys::Reflect` because the
            // `ServiceWorkerContainer` and `EventTarget` web-sys
            // features aren't enabled in `Cargo.toml`; typed access
            // would require a manifest change outside this file's scope.

            let on_message = wasm_bindgen::prelude::Closure::<dyn FnMut(_)>::new(
                move |event: web_sys::Event| {
                    let data = match js_sys::Reflect::get(
                        event.as_ref(),
                        &wasm_bindgen::JsValue::from_str("data"),
                    ) {
                        Ok(d) => d,
                        Err(_) => return,
                    };
                    let msg_type =
                        js_sys::Reflect::get(&data, &wasm_bindgen::JsValue::from_str("type"))
                            .ok()
                            .and_then(|v| v.as_string())
                            .unwrap_or_default();
                    if msg_type != "CACHE_UPDATED" {
                        return;
                    }
                    let wants_reload =
                        js_sys::Reflect::get(&data, &wasm_bindgen::JsValue::from_str("reload"))
                            .ok()
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                    if !wants_reload {
                        return;
                    }
                    let msg_version =
                        js_sys::Reflect::get(&data, &wasm_bindgen::JsValue::from_str("version"))
                            .ok()
                            .and_then(|v| v.as_string())
                            .unwrap_or_default();
                    // Defensive: skip the reload when the SW announces
                    // a version we already mirror from `/api/config`.
                    let already_known = APP_VERSION.with(|v| msg_version == v.borrow().as_str());
                    if already_known {
                        return;
                    }
                    if let Some(w) = web_sys::window() {
                        let _ = w.location().reload();
                    }
                },
            );
            let on_controllerchange =
                wasm_bindgen::prelude::Closure::<dyn FnMut(_)>::new(move |_: web_sys::Event| {
                    // `controllerchange` carries no payload, so the
                    // defensive version compare cannot apply. Trust the
                    // SW's `clients.claim()` and reload unconditionally.
                    if let Some(w) = web_sys::window() {
                        let _ = w.location().reload();
                    }
                });

            if let Some(sw_container) = js_sys::Reflect::get(
                window.navigator().as_ref(),
                &wasm_bindgen::JsValue::from_str("serviceWorker"),
            )
            .ok()
                && let Ok(add_fn) = js_sys::Reflect::get(
                    &sw_container,
                    &wasm_bindgen::JsValue::from_str("addEventListener"),
                )
                .and_then(|v| v.dyn_into::<js_sys::Function>())
            {
                let _ = add_fn.call2(
                    &sw_container,
                    &wasm_bindgen::JsValue::from_str("message"),
                    on_message.as_ref(),
                );
                let _ = add_fn.call2(
                    &sw_container,
                    &wasm_bindgen::JsValue::from_str("controllerchange"),
                    on_controllerchange.as_ref(),
                );
            }
            on_message.forget();
            on_controllerchange.forget();
        }
    }
}
