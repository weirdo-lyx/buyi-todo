use std::fs;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use tauri::Manager; // 导入 Manager trait 以使用 .path() 方法
use tauri::PhysicalPosition;
use tauri_plugin_autostart::MacosLauncher;

// 定义任务数据结构，必须派生 Serialize 和 Deserialize 才能在前端和后端之间传递
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Task {
    text: String,
    completed: bool,
}

// 获取存储文件的路径：通常放在用户的数据目录下
fn get_save_path(app_handle: tauri::AppHandle) -> PathBuf {
    let mut path = app_handle.path().app_data_dir().expect("无法获取应用数据目录");
    // 确保目录存在
    if !path.exists() {
        fs::create_dir_all(&path).unwrap();
    }
    path.push("tasks.json");
    // 打印真实路径，方便你在控制台看到并复制
    println!("========================================");
    println!("当前数据存储路径: {:?}", path);
    println!("========================================");
    path
}

// 这是一个 Tauri Command，前端可以通过 invoke("save_tasks_rust", { tasks: [...] }) 调用它
#[tauri::command]
fn save_tasks_rust(app_handle: tauri::AppHandle, tasks: Vec<Task>) -> Result<(), String> {
    let path = get_save_path(app_handle);
    let json = serde_json::to_string(&tasks).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

// 读取任务的 Command
#[tauri::command]
fn load_tasks_rust(app_handle: tauri::AppHandle) -> Result<Vec<Task>, String> {
    let path = get_save_path(app_handle);
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let tasks: Vec<Task> = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(tasks)
}

#[tauri::command]
fn move_window(window: tauri::Window, x: i32, y: i32) {
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent))
        .plugin(tauri_plugin_shell::init())
        // 注册我们写好的 Command，这样前端才能发现它们
        .invoke_handler(tauri::generate_handler![
            save_tasks_rust,
            load_tasks_rust,
            move_window
        ])
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用时出错");
}
