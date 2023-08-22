use anyhow::{anyhow, Result};
use js_sys::ArrayBuffer;
use wasm_bindgen::{
    closure::WasmClosure, closure::WasmClosureFnOnce, prelude::Closure, JsCast, JsValue,
};
use wasm_bindgen_futures::JsFuture;
use web_sys::{CanvasRenderingContext2d, Response, Document, HtmlCanvasElement, HtmlImageElement, Window};

#[allow(unused_macros)]
macro_rules! log {
    ($($t:tt)* ) => {
        web_sys::console::log_1(&format!( $($t)* ).into());
    }
}

#[allow(unused_macros)]
macro_rules! error {
    ($($t:tt)* ) => {
        web_sys::console::error_1(&format!( $($t)* ).into());
    }
}

pub fn window() -> Result<Window> {
    web_sys::window().ok_or_else(|| anyhow!("No window found"))
}
pub fn document() -> Result<Document> {
    window()?
        .document()
        .ok_or_else(|| anyhow!("No document found"))
}
pub fn canvas() -> Result<HtmlCanvasElement> {
    document()?
        .get_element_by_id("canvas")
        .ok_or_else(|| anyhow!("No canvas found with ID canvas"))?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|elem| anyhow!("Failed to cast {:#?} to HtmlCanvasElement", elem))
}

pub fn context() -> Result<CanvasRenderingContext2d> {
    canvas()?
        .get_context("2d")
        .map_err(|_| anyhow!("Failed to get 2d context"))?
        .ok_or_else(|| anyhow!("Failed to get 2d context"))?
        .dyn_into::<CanvasRenderingContext2d>()
        .map_err(|ctx| anyhow!("Failed to cast {:#?} to CanvasRenderingContext2d", ctx))
}

pub fn spawn_local<F>(future: F)
where
    F: futures::Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future)
}

pub async fn fetch_with_str(resource: &str) -> Result<JsValue> {
    JsFuture::from(window()?.fetch_with_str(resource))
        .await
        .map_err(|_| anyhow!("Failed to fetch {}", resource))
}

pub async fn fetch_response(resource: &str) -> Result<Response> {
    fetch_with_str(resource)
        .await?
        .dyn_into::<Response>()
        .map_err(|err| anyhow!("Failed to cast response to web_sys::Response: {:#?}", err))
}

pub async fn fetch_json(json_path: &str) -> Result<JsValue> {
    let resp = fetch_response(json_path).await?;
    
    JsFuture::from(
        resp.json()
            .map_err(|err| anyhow!("Failed to make json: {:#?}", err))?,
    )
    .await
    .map_err(|err| anyhow!("Failed to make json: {:#?}", err))
}

pub async fn fetch_array_buffer(resource: &str) -> Result<ArrayBuffer> {
    let resp = fetch_response(resource).await?;
    JsFuture::from(
        resp.array_buffer()
            .map_err(|err| anyhow!("Failed to make array buffer: {:#?}", err))?,
    )
    .await
    .map_err(|err| anyhow!("Failed to make array buffer: {:#?}", err))?
    .dyn_into::<ArrayBuffer>()
    .map_err(|err| anyhow!("Failed to cast array buffer: {:#?}", err))
}

pub fn new_image() -> Result<HtmlImageElement> {
    HtmlImageElement::new().map_err(|_| anyhow!("Failed to create image"))
}
pub fn closure_once<F, A, R>(fn_once: F) -> Closure<F::FnMut>
where
    F: 'static + WasmClosureFnOnce<A, R>,
{
    Closure::once(fn_once)
}

pub type LoopClosure = Closure<dyn FnMut(f64)>;
pub fn request_animation_frame(f: &LoopClosure) -> Result<i32> {
    window()?
        .request_animation_frame(f.as_ref().unchecked_ref())
        .map_err(|_| anyhow!("Failed to request animation frame"))
}

pub fn closure_wrap<T: WasmClosure + ?Sized>(data: Box<T>) -> Closure<T> {
    Closure::wrap(data)
}

pub fn create_raf_closure(f: impl FnMut(f64) + 'static) -> LoopClosure {
    closure_wrap(Box::new(f))
}

pub fn now() -> Result<f64> {
    Ok(window()?
        .performance()
        .ok_or_else(|| anyhow!("No performance found"))?
        .now())
}
