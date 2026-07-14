use std::path::PathBuf;
use std::process::Command;

fn main() {
    // 必须在 tauri_build 之前执行，以生成资源清单
    compile_slime_calculator();
    tauri_build::build();
}

/// 从 GitHub Releases 下载 slime-calculator 二进制文件
fn compile_slime_calculator() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let bin_dir = manifest_dir.join("bin");

    // 确保 bin 目录存在
    let _ = std::fs::create_dir_all(&bin_dir);

    // 判断平台后缀
    let exe_ext = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };

    let repo = "minelogy-dev/slime-calculator";
    let targets = ["slime_main", "slime_circle", "slime_cmp"];

    for name in &targets {
        let output = bin_dir.join(format!("{}{}", name, exe_ext));
        if output.exists() {
            println!("cargo:warning=✅ {} 已存在，跳过", name);
            continue;
        }

        if download_from_github(repo, &bin_dir, name, exe_ext) {
            println!("cargo:warning=✅ 从 GitHub Releases 下载 {} 成功", name);
        } else {
            println!("cargo:warning=❌ 从 GitHub Releases 下载 {} 失败", name);
        }
    }

    // 列出最终产物
    if let Ok(entries) = std::fs::read_dir(&bin_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                let name = entry.file_name().to_string_lossy().to_string();
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                println!("cargo:warning=📦 打包: {} ({} bytes)", name, size);
            }
        }
    }
}

/// 从 GitHub Releases 下载二进制文件
fn download_from_github(repo: &str, bin_dir: &PathBuf, name: &str, exe_ext: &str) -> bool {
    let output_path = bin_dir.join(format!("{}{}", name, exe_ext));
    let download_url = format!(
        "https://github.com/{repo}/releases/latest/download/{name}{exe_ext}"
    );

    // macOS / Linux 使用 curl
    if !cfg!(target_os = "windows") {
        let status = Command::new("curl")
            .args([
                "-fsSL",
                "-o",
                output_path.to_str().unwrap(),
                &download_url,
            ])
            .status();
        if let Ok(s) = status {
            if s.success() {
                // 保留可执行权限
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(
                    &output_path,
                    std::fs::Permissions::from_mode(0o755),
                );
                return true;
            }
        }
    }

    // Windows 使用 PowerShell
    if cfg!(target_os = "windows") {
        let status = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Invoke-WebRequest -Uri '{}' -OutFile '{}'",
                    download_url,
                    output_path.display()
                ),
            ])
            .status();
        if let Ok(s) = status {
            return s.success();
        }
    }

    false
}
