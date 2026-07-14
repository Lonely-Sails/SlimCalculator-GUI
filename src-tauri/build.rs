use std::path::PathBuf;

fn main() {
    // 必须在 tauri_build 之前执行，以生成资源清单
    verify_binaries();
    tauri_build::build();
}

/// 检查当前平台需要的二进制文件是否存在，并清理其他平台的残留
fn verify_binaries() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let bin_dir = manifest_dir.join("bin");

    // 当前平台的后缀
    let this_ext = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };
    let other_ext = if cfg!(target_os = "windows") {
        ""
    } else {
        ".exe"
    };

    let _ = std::fs::create_dir_all(&bin_dir);

    let targets = ["slime_main", "slime_circle", "slime_cmp"];

    for name in &targets {
        // 清理其他平台的文件
        let other = bin_dir.join(format!("{}{}", name, other_ext));
        if other.exists() {
            let _ = std::fs::remove_file(&other);
            println!("cargo:warning=🧹 清理其他平台文件: {}{}", name, other_ext);
        }

        // 检查当前平台的文件
        let path = bin_dir.join(format!("{}{}", name, this_ext));
        if path.exists() {
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            println!("cargo:warning=✅ {} ({})", name, format_size(size));
        } else {
            println!("cargo:warning=❌ {} 未找到，请先在 CI 中下载或手动放入 bin/", name);
        }
    }
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
