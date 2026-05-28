#[cfg(target_arch = "wasm32")]
static DIRTY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(target_arch = "wasm32")]
pub fn set_unload_guard(dirty: bool) {
    DIRTY.store(dirty, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(target_arch = "wasm32")]
pub fn install_unload_guard() {
    use wasm_bindgen::prelude::*;
    use web_sys::BeforeUnloadEvent;

    let closure = Closure::<dyn Fn(BeforeUnloadEvent)>::new(|e: BeforeUnloadEvent| {
        if DIRTY.load(std::sync::atomic::Ordering::Relaxed) {
            e.prevent_default();
            e.set_return_value("저장되지 않은 변경사항이 있습니다.");
        }
    });

    if let Some(window) = web_sys::window() {
        window
            .add_event_listener_with_callback("beforeunload", closure.as_ref().unchecked_ref())
            .ok();
    }
    closure.forget();
}

#[cfg(target_arch = "wasm32")]
pub fn set_page_title(title: &str) {
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        document.set_title(title);
    }
}

#[cfg(target_arch = "wasm32")]
pub fn trigger_download(data: &[u8], filename: &str) {
    use js_sys::{Array, Uint8Array};
    use wasm_bindgen::JsCast;
    use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

    let array = Uint8Array::from(data);
    let blob_parts = Array::new();
    blob_parts.push(&array);

    let opts = BlobPropertyBag::new();
    opts.set_type("application/octet-stream");
    let Ok(blob) = Blob::new_with_u8_array_sequence_and_options(&blob_parts, &opts) else {
        log::error!("Blob 생성 실패");
        return;
    };

    let Ok(url) = Url::create_object_url_with_blob(&blob) else {
        log::error!("Object URL 생성 실패");
        return;
    };

    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };

    if let Ok(elem) = document.create_element("a") {
        if let Ok(a) = elem.dyn_into::<HtmlAnchorElement>() {
            a.set_href(&url);
            a.set_download(filename);
            a.click();
        }
    }

    Url::revoke_object_url(&url).ok();
}
