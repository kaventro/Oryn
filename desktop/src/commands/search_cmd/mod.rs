mod core;

pub use core::SearchService;
use core::{
    SearchAckOut, SearchCancelIn, SearchPageIn, SearchPageOut, SearchSessionIn, SearchStartIn,
    SearchStartOut,
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn search_clear_cache(service: State<'_, Arc<SearchService>>) -> SearchAckOut {
    service.clear()
}

#[tauri::command]
pub async fn search_start(
    service: State<'_, Arc<SearchService>>,
    input: SearchStartIn,
) -> Result<SearchStartOut, String> {
    service.start(input).await
}

#[tauri::command]
pub fn search_get_page(
    service: State<'_, Arc<SearchService>>,
    input: SearchPageIn,
) -> Result<SearchPageOut, String> {
    service.page(input)
}

#[tauri::command]
pub fn search_cancel(
    service: State<'_, Arc<SearchService>>,
    input: SearchCancelIn,
) -> SearchAckOut {
    service.cancel(input)
}

#[tauri::command]
pub fn search_release(
    service: State<'_, Arc<SearchService>>,
    input: SearchSessionIn,
) -> SearchAckOut {
    service.release(input)
}
