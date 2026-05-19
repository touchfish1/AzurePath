use crate::core::log_buffer;

#[tauri::command]
pub fn get_logs(count: Option<usize>) -> Vec<log_buffer::LogEntry> {
    log_buffer::get_logs(count.unwrap_or(200))
}

#[tauri::command]
pub fn clear_logs() {
    log_buffer::clear_logs();
}
