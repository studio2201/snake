//! RAII wrapper around `EventTarget::add_event_listener`.
//!
//! Dropping the [`EventListener`] detaches the listener, so callers don't
//! have to keep track of the underlying [`Closure`] manually. Used by
//! [`super::snake::keys`] to install and tear down keyboard handlers as the
//! game state changes.

use wasm_bindgen::prelude::*;

/// Owns a `Closure` plus the [`web_sys::EventTarget`] it is attached to.
///
/// The closure is kept alive for the lifetime of this struct; on [`Drop`]
/// the listener is removed and the closure is dropped, releasing the
/// reference the JS side held on it.
pub struct EventListener {
    target: web_sys::EventTarget,
    event_type: &'static str,
    closure: Option<wasm_bindgen::closure::Closure<dyn FnMut(web_sys::Event)>>,
}

impl EventListener {
    /// Attaches `callback` to `event_type` on `target`.
    ///
    /// The callback is wrapped in a boxed `Closure<dyn FnMut>` so that it
    /// can be passed to `add_event_listener_with_callback`. Panics if the
    /// underlying DOM call returns an error, which only happens for
    /// invalid event types (caught at compile time via the `&'static str`
    /// parameter convention) or detached targets.
    pub fn new<F>(target: &web_sys::EventTarget, event_type: &'static str, callback: F) -> Self
    where
        F: FnMut(web_sys::Event) + 'static,
    {
        let closure = Closure::wrap(Box::new(callback) as Box<dyn FnMut(web_sys::Event)>);
        target
            .add_event_listener_with_callback(event_type, closure.as_ref().unchecked_ref())
            .expect("failed to attach event listener");
        Self {
            target: target.clone(),
            event_type,
            closure: Some(closure),
        }
    }
}

impl Drop for EventListener {
    fn drop(&mut self) {
        if let Some(closure) = self.closure.take() {
            let _ = self.target.remove_event_listener_with_callback(
                self.event_type,
                closure.as_ref().unchecked_ref(),
            );
        }
    }
}
