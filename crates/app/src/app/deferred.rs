use std::sync::{Arc, Mutex};

pub struct Deferred<T>(Arc<Mutex<Option<T>>>);

impl<T: Send + 'static> Deferred<T> {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn_thread<F: FnOnce() -> T + Send + 'static>(f: F) -> Self {
        let arc = Arc::new(Mutex::new(None));
        let inner = arc.clone();
        std::thread::spawn(move || {
            *inner.lock().unwrap() = Some(f());
        });
        Self(arc)
    }
}

impl<T: 'static> Deferred<T> {
    #[cfg(target_arch = "wasm32")]
    pub fn spawn_async<F: std::future::Future<Output = T> + 'static>(future: F, ctx: eframe::egui::Context) -> Self {
        let arc = Arc::new(Mutex::new(None));
        let inner = arc.clone();
        wasm_bindgen_futures::spawn_local(async move {
            *inner.lock().unwrap() = Some(future.await);
            ctx.request_repaint();
        });
        Self(arc)
    }

    pub fn take_ready(&self) -> Option<T> {
        self.0.lock().ok()?.take()
    }
}
