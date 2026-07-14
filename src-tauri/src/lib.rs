use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, State};

// ── 数据结构 ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlimeRecord {
    pub x: i64,
    pub z: i64,
    pub slime_count: i64,
    pub distance: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineParams {
    pub seed: i64,
    pub start_x: i32,
    pub start_z: i32,
    pub end_x: i32,
    pub end_z: i32,
    pub size_x: i32,
    pub size_z: i32,
    pub threshold: i32,
    pub circle_radius: Option<i32>,
    pub cmp_distance: Option<i32>,
    pub cmp_threshold: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinariesStatus {
    pub slime_main: bool,
    pub slime_circle: bool,
    pub slime_cmp: bool,
    pub bin_dir: String,
}

pub struct AppState {
    pub bin_dir: Mutex<PathBuf>,
}

// ── 辅助函数 ──────────────────────────────────────────────

fn find_bin_dir() -> PathBuf {
    // 可执行文件同目录下的 libs/ 文件夹
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let libs = exe_dir.join("libs");
            if libs.join("slime_main").exists() {
                return libs;
            }
            // 开发模式下直接在同目录下找
            if exe_dir.join("slime_main").exists() {
                return exe_dir.to_path_buf();
            }
            return libs;
        }
    }
    PathBuf::from("libs")
}

fn check_binary(path: &PathBuf, name: &str) -> bool {
    let bin_path = if cfg!(target_os = "windows") {
        path.join(format!("{}.exe", name))
    } else {
        path.join(name)
    };
    bin_path.exists()
}

fn run_binary(
    bin_dir: &PathBuf,
    name: &str,
    args: &[String],
) -> Result<String, String> {
    let bin_path = if cfg!(target_os = "windows") {
        bin_dir.join(format!("{}.exe", name))
    } else {
        bin_dir.join(name)
    };

    if !bin_path.exists() {
        return Err(format!(
            "❌ 找不到 {}。\n预期路径: {}\n\
             请将 {} 放在可执行文件旁 libs/ 目录下。",
            name,
            bin_path.display(),
            name,
        ));
    }

    let output = Command::new(&bin_path)
        .args(args)
        .output()
        .map_err(|e| format!("执行 {} 失败: {}", name, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("{} 执行失败:\n{}\n{}", name, stdout, stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_main_csv(content: &str) -> Result<Vec<SlimeRecord>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(content.as_bytes());

    let mut records = Vec::new();
    for result in reader.deserialize() {
        #[derive(Deserialize)]
        struct Row {
            x: i64,
            z: i64,
            slime_count: i64,
        }
        match result {
            Ok(row) => {
                let row: Row = row;
                records.push(SlimeRecord {
                    x: row.x,
                    z: row.z,
                    slime_count: row.slime_count,
                    distance: None,
                });
            }
            Err(e) => eprintln!("CSV 解析警告: {}", e),
        }
    }
    Ok(records)
}

fn parse_cmp_csv(content: &str) -> Result<Vec<SlimeRecord>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(content.as_bytes());

    let mut records = Vec::new();
    for result in reader.deserialize() {
        #[derive(Deserialize)]
        struct Row {
            x: i64,
            z: i64,
            slime_count: i64,
            distance: f64,
        }
        match result {
            Ok(row) => {
                let row: Row = row;
                records.push(SlimeRecord {
                    x: row.x,
                    z: row.z,
                    slime_count: row.slime_count,
                    distance: Some(row.distance),
                });
            }
            Err(e) => eprintln!("CSV 解析警告: {}", e),
        }
    }
    Ok(records)
}

// ── Tauri 命令 ────────────────────────────────────────────

#[tauri::command]
fn get_binaries_status(state: State<AppState>) -> BinariesStatus {
    let bin_dir = state.bin_dir.lock().unwrap().clone();
    BinariesStatus {
        slime_main: check_binary(&bin_dir, "slime_main"),
        slime_circle: check_binary(&bin_dir, "slime_circle"),
        slime_cmp: check_binary(&bin_dir, "slime_cmp"),
        bin_dir: bin_dir.display().to_string(),
    }
}

fn get_tmp_dir() -> PathBuf {
    std::env::temp_dir().join(format!("slime_calc_{}", std::process::id()))
}

/// 运行 slime_main，返回解析后的记录
#[tauri::command]
async fn run_slime_main(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    seed: i64,
    start_x: i32,
    start_z: i32,
    end_x: i32,
    end_z: i32,
    size_x: i32,
    size_z: i32,
    threshold: i32,
) -> Result<Vec<SlimeRecord>, String> {
    let bin_dir = state.bin_dir.lock().unwrap().clone();
    let tmp_dir = get_tmp_dir();
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("创建临时目录失败: {}", e))?;

    let output_path = tmp_dir.join("main_output.csv");
    let output_str = output_path.to_str().unwrap().to_string();

    let _ = app_handle.emit("pipeline-progress", serde_json::json!({
        "step": "slime_main", "message": "🚀 启动 slime_main 扫描器...", "percent": 10.0
    }));

    let args = vec![
        seed.to_string(),
        start_x.to_string(),
        start_z.to_string(),
        end_x.to_string(),
        end_z.to_string(),
        size_x.to_string(),
        size_z.to_string(),
        threshold.to_string(),
        output_str,
    ];

    run_binary(&bin_dir, "slime_main", &args)?;

    let _ = app_handle.emit("pipeline-progress", serde_json::json!({
        "step": "slime_main", "message": "📖 读取扫描结果...", "percent": 40.0
    }));

    let content = std::fs::read_to_string(&output_path)
        .map_err(|e| format!("读取 slime_main 输出失败: {}", e))?;

    let records = parse_main_csv(&content)?;
    let _ = std::fs::remove_dir_all(&tmp_dir);
    Ok(records)
}

/// 运行 slime_circle，返回解析后的记录
#[tauri::command]
async fn run_slime_circle(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    input_csv: String,
    radius: i32,
    size_x: i32,
    size_z: i32,
    seed: i64,
    threshold: i32,
) -> Result<Vec<SlimeRecord>, String> {
    let bin_dir = state.bin_dir.lock().unwrap().clone();
    let tmp_dir = get_tmp_dir();
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("创建临时目录失败: {}", e))?;

    let input_path = tmp_dir.join("input.csv");
    let output_path = tmp_dir.join("circle_output.csv");
    std::fs::write(&input_path, &input_csv).map_err(|e| format!("写入输入文件失败: {}", e))?;

    let _ = app_handle.emit("pipeline-progress", serde_json::json!({
        "step": "slime_circle", "message": "🔄 运行圆形区域筛选...", "percent": 50.0
    }));

    let args = vec![
        input_path.to_str().unwrap().to_string(),
        radius.to_string(),
        size_x.to_string(),
        size_z.to_string(),
        seed.to_string(),
        output_path.to_str().unwrap().to_string(),
        threshold.to_string(),
    ];

    run_binary(&bin_dir, "slime_circle", &args)?;

    let content = std::fs::read_to_string(&output_path)
        .map_err(|e| format!("读取 slime_circle 输出失败: {}", e))?;
    let records = parse_main_csv(&content)?;
    let _ = std::fs::remove_dir_all(&tmp_dir);
    Ok(records)
}

/// 运行 slime_cmp，返回解析后的记录（含距离信息）
#[tauri::command]
async fn run_slime_cmp(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    input_csv: String,
    distance: i32,
    threshold: i32,
) -> Result<Vec<SlimeRecord>, String> {
    let bin_dir = state.bin_dir.lock().unwrap().clone();
    let tmp_dir = get_tmp_dir();
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("创建临时目录失败: {}", e))?;

    let input_path = tmp_dir.join("input.csv");
    let output_path = tmp_dir.join("cmp_output.csv");
    std::fs::write(&input_path, &input_csv).map_err(|e| format!("写入输入文件失败: {}", e))?;

    let _ = app_handle.emit("pipeline-progress", serde_json::json!({
        "step": "slime_cmp", "message": "📏 运行距离筛选...", "percent": 80.0
    }));

    let args = vec![
        input_path.to_str().unwrap().to_string(),
        distance.to_string(),
        threshold.to_string(),
        output_path.to_str().unwrap().to_string(),
    ];

    run_binary(&bin_dir, "slime_cmp", &args)?;

    let content = std::fs::read_to_string(&output_path)
        .map_err(|e| format!("读取 slime_cmp 输出失败: {}", e))?;
    let records = parse_cmp_csv(&content)?;
    let _ = std::fs::remove_dir_all(&tmp_dir);
    Ok(records)
}

/// 运行完整流水线（slime_main → slime_circle(可选) → slime_cmp(可选)）
#[tauri::command]
async fn run_pipeline(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    params: PipelineParams,
) -> Result<Vec<SlimeRecord>, String> {
    let bin_dir = state.bin_dir.lock().unwrap().clone();
    let tmp_dir = get_tmp_dir();
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("创建临时目录失败: {}", e))?;

    // ── Step 1: slime_main ──
    let _ = app_handle.emit("pipeline-progress", serde_json::json!({
        "step": "slime_main", "message": "🚀 启动 slime_main 扫描器...", "percent": 5.0
    }));

    let main_output = tmp_dir.join("step1_main.csv");
    let args_main = vec![
        params.seed.to_string(),
        params.start_x.to_string(),
        params.start_z.to_string(),
        params.end_x.to_string(),
        params.end_z.to_string(),
        params.size_x.to_string(),
        params.size_z.to_string(),
        params.threshold.to_string(),
        main_output.to_str().unwrap().to_string(),
    ];
    run_binary(&bin_dir, "slime_main", &args_main)?;

    let main_content = std::fs::read_to_string(&main_output)
        .map_err(|e| format!("读取 slime_main 输出失败: {}", e))?;

    // 如果只运行 main，直接返回
    if params.circle_radius.is_none() && params.cmp_distance.is_none() {
        let _ = app_handle.emit("pipeline-progress", serde_json::json!({
            "step": "done", "message": "✅ 扫描完成！", "percent": 100.0
        }));
        let records = parse_main_csv(&main_content)?;
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Ok(records);
    }

    // ── Step 2: slime_circle (可选) ──
    let circle_content = if let Some(radius) = params.circle_radius {
        let _ = app_handle.emit("pipeline-progress", serde_json::json!({
            "step": "slime_circle", "message": "🔄 运行圆形区域筛选...", "percent": 35.0
        }));

        let circle_output = tmp_dir.join("step2_circle.csv");
        let circle_input = tmp_dir.join("circle_input.csv");
        std::fs::write(&circle_input, &main_content)
            .map_err(|e| format!("写入 circle 输入失败: {}", e))?;

        let args_circle = vec![
            circle_input.to_str().unwrap().to_string(),
            radius.to_string(),
            params.size_x.to_string(),
            params.size_z.to_string(),
            params.seed.to_string(),
            circle_output.to_str().unwrap().to_string(),
            params.threshold.to_string(),
        ];
        run_binary(&bin_dir, "slime_circle", &args_circle)?;

        std::fs::read_to_string(&circle_output)
            .map_err(|e| format!("读取 slime_circle 输出失败: {}", e))?
    } else {
        main_content
    };

    // ── Step 3: slime_cmp (可选) ──
    let final_content = if let (Some(dist), Some(cmp_thr)) =
        (params.cmp_distance, params.cmp_threshold)
    {
        let _ = app_handle.emit("pipeline-progress", serde_json::json!({
            "step": "slime_cmp", "message": "📏 运行距离筛选...", "percent": 70.0
        }));

        let cmp_output = tmp_dir.join("step3_cmp.csv");
        let cmp_input = tmp_dir.join("cmp_input.csv");
        std::fs::write(&cmp_input, &circle_content)
            .map_err(|e| format!("写入 cmp 输入失败: {}", e))?;

        let args_cmp = vec![
            cmp_input.to_str().unwrap().to_string(),
            dist.to_string(),
            cmp_thr.to_string(),
            cmp_output.to_str().unwrap().to_string(),
        ];
        run_binary(&bin_dir, "slime_cmp", &args_cmp)?;

        std::fs::read_to_string(&cmp_output)
            .map_err(|e| format!("读取 slime_cmp 输出失败: {}", e))?
    } else {
        circle_content
    };

    let _ = app_handle.emit("pipeline-progress", serde_json::json!({
        "step": "done", "message": "✅ 全部完成！", "percent": 100.0
    }));

    let records = if params.cmp_distance.is_some() {
        parse_cmp_csv(&final_content)?
    } else {
        parse_main_csv(&final_content)?
    };

    let _ = std::fs::remove_dir_all(&tmp_dir);
    Ok(records)
}

/// 导出结果为 CSV 文件
#[tauri::command]
fn export_csv(records: Vec<SlimeRecord>, file_path: String) -> Result<(), String> {
    let mut wtr = csv::Writer::from_path(&file_path)
        .map_err(|e| format!("创建 CSV 文件失败: {}", e))?;

    let has_distance = records.iter().any(|r| r.distance.is_some());

    if has_distance {
        wtr.write_record(&["x", "z", "slime_count", "distance"])
            .map_err(|e| format!("写入表头失败: {}", e))?;
        for r in &records {
            wtr.write_record(&[
                r.x.to_string(),
                r.z.to_string(),
                r.slime_count.to_string(),
                r.distance.map(|d| format!("{:.6}", d)).unwrap_or_default(),
            ])
            .map_err(|e| format!("写入记录失败: {}", e))?;
        }
    } else {
        wtr.write_record(&["x", "z", "slime_count"])
            .map_err(|e| format!("写入表头失败: {}", e))?;
        for r in &records {
            wtr.write_record(&[
                r.x.to_string(),
                r.z.to_string(),
                r.slime_count.to_string(),
            ])
            .map_err(|e| format!("写入记录失败: {}", e))?;
        }
    }

    wtr.flush().map_err(|e| format!("刷新 CSV 失败: {}", e))?;
    Ok(())
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ── 应用入口 ──────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            bin_dir: Mutex::new(PathBuf::new()),
        })
        .invoke_handler(tauri::generate_handler![
            get_binaries_status,
            run_slime_main,
            run_slime_circle,
            run_slime_cmp,
            run_pipeline,
            export_csv,
            get_app_version,
        ])
        .setup(|app| {
            let bin_dir = find_bin_dir();
            let state = app.state::<AppState>();
            *state.bin_dir.lock().unwrap() = bin_dir;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
