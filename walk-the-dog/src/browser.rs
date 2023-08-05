use anyhow::{anyhow, Result};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen::closure::{Closure, WasmClosureFnOnce};
use wasm_bindgen_futures::JsFuture;
use web_sys::{CanvasRenderingContext2d, Document, HtmlCanvasElement, Window, HtmlImageElement};

macro_rules! log {
    ($($t:tt)* ) => {
        web_sys::console::log_1(&format!( $($t)* ).into());
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
pub async fn fetch_json(json_path: &str) -> Result<JsValue> {
    let resp_value = fetch_with_str(json_path).await?;
    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| anyhow!("Failed to cast response to web_sys::Response"))?;
    JsFuture::from(
        resp.json()
            .map_err(|err| anyhow!("Failed to make json: {:#?}", err))?,
    )
    .await
    .map_err(|err| anyhow!("Failed to make json: {:#?}", err))
}
pub fn new_image() -> Result<HtmlImageElement> {
    HtmlImageElement::new().map_err(|_| anyhow!("Failed to create image"))
}
pub fn closure_once<F, A, R>(fn_once: F) -> Closure<F::FnMut>
where
    F:'static + WasmClosureFnOnce<A, R>,
{
    Closure::once(fn_once)
}
