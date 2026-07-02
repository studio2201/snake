use crate::app::{App, Msg};
use crate::components::snake_game::SnakeGame;
use crate::components::header::Header;
use crate::components::pin::Login;
use shared_core::i18n::Language;
use yew::prelude::*;

impl App {
    pub fn view_app(&self, ctx: &Context<Self>) -> Html {
        let locale_on_change = {
            let link = ctx.link().clone();
            Callback::from(move |new_lang: String| {
                link.send_message(Msg::SwitchLanguage(new_lang));
            })
        };
        let locale_context = crate::i18n::LocaleContext {
            current: self.locale_state.clone(),
            on_change: locale_on_change,
        };

        let toggle_theme = ctx.link().callback(|_| Msg::ToggleTheme);

        let on_logout = ctx.link().callback(|_| Msg::Logout);



        let content_class = if self.authenticated {
            "app-body"
        } else {
            "container"
        };

        html! {
            <ContextProvider<crate::i18n::LocaleContext> context={locale_context}>
                <Header
                    site_title={self.site_title.clone()}
                    theme={self.theme.clone()}
                    language={Language::from_code(&self.locale_state)}
                    toggle_theme={toggle_theme}
                    on_language_change={
                        let link = ctx.link().clone();
                        Callback::from(move |lang: Language| {
                            link.send_message(Msg::SwitchLanguage(lang.code().to_string()));
                        })
                    }
                    is_authenticated={self.authenticated}
                    pin_required={self.is_pin_required}
                    on_logout={on_logout}
                    on_print={Some(ctx.link().callback(|_| Msg::Print))}
                    print_disabled={self.is_pin_required && !self.authenticated}
                    enable_translation={self.enable_translation}
                    enable_themes={self.enable_themes}
                    enable_print={self.enable_print}
                    version={Some(self.app_version.clone())}
                />
                <div class={content_class}>
                    {if !self.authenticated {
                        html! { <Login on_login_success={
                            let link = ctx.link().clone();
                            Callback::from(move |_| {
                                link.send_message(Msg::SetAuthenticated(true));
                                if let Some(win) = web_sys::window() {
                                    let loc = win.location();
                                    let search = loc.search().unwrap_or_default();
                                    let mut redirect_url = "/".to_string();
                                    if let Ok(params) = web_sys::UrlSearchParams::new_with_str(&search)
                                        && let Some(r) = params.get("redirect")
                                            && !r.is_empty() && r.starts_with('/') && !r.starts_with("//") {
                                                redirect_url = r;
                                            }
                                    if let Ok(history) = win.history() {
                                        let _ = history.replace_state_with_url(
                                            &wasm_bindgen::JsValue::NULL,
                                            "",
                                            Some(&redirect_url),
                                        );
                                    }
                                }
                            })
                        }
                        on_status_change={
                            let link = ctx.link().clone();
                            Callback::from(move |status| link.send_message(Msg::SetStatus(status)))
                        } /> }
                    } else {
                        html! {
                            <main>
                                <SnakeGame
                                    on_status={
                                        let link = ctx.link().clone();
                                        Callback::from(move |status| link.send_message(Msg::SetStatus(status)))
                                    }
                                />
                            </main>
                        }
                    }}
                </div>
                <crate::components::footer::Footer version={self.app_version.clone()} show_github={self.show_github}>
                    {
                        if let Some((msg, cls)) = &self.active_notification {
                            html! { <div class={format!("footer-status-text {}", cls)}>{ msg }</div> }
                        } else {
                            html! { <div class="footer-status-text success">{"Ready"}</div> }
                        }
                    }
                </crate::components::footer::Footer>
            </ContextProvider<crate::i18n::LocaleContext>>
        }
    }
}
