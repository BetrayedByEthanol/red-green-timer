use crate::application::{AppState, CompletedRunSummaryDto, RunHistoryDto, TimerDto, TimerRequest};
use tauri::State;
use timer_core::TimerSnapshot;
use uuid::Uuid;

#[tauri::command]
pub async fn list_timers(state: State<'_, AppState>) -> Result<Vec<TimerDto>, String> {
    state
        .lock()
        .await
        .list_timers()
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn create_timer(
    state: State<'_, AppState>,
    request: TimerRequest,
) -> Result<TimerDto, String> {
    state
        .lock()
        .await
        .create_timer(request)
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn update_timer(
    state: State<'_, AppState>,
    timer_id: Uuid,
    request: TimerRequest,
) -> Result<TimerDto, String> {
    state
        .lock()
        .await
        .update_timer(timer_id, request)
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn archive_timer(state: State<'_, AppState>, timer_id: Uuid) -> Result<(), String> {
    state
        .lock()
        .await
        .archive_timer(timer_id)
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn start_timer(
    state: State<'_, AppState>,
    timer_id: Uuid,
) -> Result<TimerSnapshot, String> {
    state
        .lock()
        .await
        .start_timer(timer_id)
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn stop_green(state: State<'_, AppState>) -> Result<TimerSnapshot, String> {
    state.lock().await.stop_green().map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn stop_run(state: State<'_, AppState>) -> Result<CompletedRunSummaryDto, String> {
    state
        .lock()
        .await
        .stop_run()
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn get_timer_snapshot(state: State<'_, AppState>) -> Result<TimerSnapshot, String> {
    Ok(state.lock().await.snapshot())
}
#[tauri::command]
pub async fn tick_timer(state: State<'_, AppState>) -> Result<TimerSnapshot, String> {
    state.lock().await.tick().map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn list_recent_runs(
    state: State<'_, AppState>,
    timer_id: Option<Uuid>,
    limit: Option<u32>,
) -> Result<Vec<RunHistoryDto>, String> {
    state
        .lock()
        .await
        .list_recent_runs(timer_id, limit)
        .await
        .map_err(|e| e.to_string())
}
