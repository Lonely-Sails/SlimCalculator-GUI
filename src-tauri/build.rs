use std::path::PathBuf;
use std::process::Command;

fn main() {
    // 必须在 tauri_build 之前执行，以生成资源清单
    compile_slime_calculator();
    tauri_build::build();
}

/// 编译 slime-calculator 的 C/CUDA 程序，并将产物复制到 src-tauri/bin/
fn compile_slime_calculator() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let slime_dir = manifest_dir.parent().unwrap().join("slime-calculator");
    let bin_dir = manifest_dir.join("bin");

    // 确保 bin 目录存在
    let _ = std::fs::create_dir_all(&bin_dir);

    // 判断平台后缀
    let (exe_ext, _) = if cfg!(target_os = "windows") {
        (".exe", "windows")
    } else {
        ("", "unix")
    };

    // ── 编译 slime_cmp (纯 C，用 gcc/cc) ──
    let cmp_src = slime_dir.join("slime_cmp.c");
    let cmp_out = bin_dir.join(format!("slime_cmp{}", exe_ext));
    if cmp_src.exists() {
        let cc = which_cc();
        let status = Command::new(&cc)
            .args([
                "-o",
                cmp_out.to_str().unwrap(),
                cmp_src.to_str().unwrap(),
                "-lm",
                "-O3",
                "-march=native",
            ])
            .status();

        match status {
            Ok(s) if s.success() => {
                println!("cargo:warning=✅ 编译 slime_cmp 成功");
            }
            Ok(s) => {
                println!(
                    "cargo:warning=⚠️  slime_cmp 编译失败 (exit: {}), 将尝试从 build/ 复制",
                    s
                );
                copy_from_build(&slime_dir, &bin_dir, "slime_cmp", exe_ext);
            }
            Err(e) => {
                println!("cargo:warning=⚠️  slime_cmp 编译失败: {}, 将尝试从 build/ 复制", e);
                copy_from_build(&slime_dir, &bin_dir, "slime_cmp", exe_ext);
            }
        }
    } else {
        println!("cargo:warning=⚠️  未找到 slime_cmp.c，尝试从 build/ 复制");
        copy_from_build(&slime_dir, &bin_dir, "slime_cmp", exe_ext);
    }

    // ── 编译 slime_main (CUDA) ──
    let main_src = slime_dir.join("slime_main.cu");
    let main_out = bin_dir.join(format!("slime_main{}", exe_ext));
    if main_src.exists() {
        if let Ok(nvcc) = which_nvcc() {
            let status = Command::new(&nvcc)
                .args([
                    "-o",
                    main_out.to_str().unwrap(),
                    main_src.to_str().unwrap(),
                    "-O3",
                    "-use_fast_math",
                    "-arch=native",
                ])
                .status();

            match status {
                Ok(s) if s.success() => {
                    println!("cargo:warning=✅ 编译 slime_main 成功");
                }
                _ => {
                    println!("cargo:warning=⚠️  slime_main CUDA 编译失败，尝试从 build/ 复制");
                    copy_from_build(&slime_dir, &bin_dir, "slime_main", exe_ext);
                }
            }
        } else {
            println!("cargo:warning=⚠️  未找到 nvcc，尝试从 build/ 复制 slime_main");
            copy_from_build(&slime_dir, &bin_dir, "slime_main", exe_ext);
        }
    } else {
        println!("cargo:warning=⚠️  未找到 slime_main.cu，尝试从 build/ 复制");
        copy_from_build(&slime_dir, &bin_dir, "slime_main", exe_ext);
    }

    // ── 编译 slime_circle (CUDA) ──
    let circle_src = slime_dir.join("slime_circle.cu");
    let circle_out = bin_dir.join(format!("slime_circle{}", exe_ext));
    if circle_src.exists() {
        if let Ok(nvcc) = which_nvcc() {
            let status = Command::new(&nvcc)
                .args([
                    "-o",
                    circle_out.to_str().unwrap(),
                    circle_src.to_str().unwrap(),
                    "-O3",
                    "-use_fast_math",
                    "-arch=native",
                ])
                .status();

            match status {
                Ok(s) if s.success() => {
                    println!("cargo:warning=✅ 编译 slime_circle 成功");
                }
                _ => {
                    println!("cargo:warning=⚠️  slime_circle CUDA 编译失败，尝试从 build/ 复制");
                    copy_from_build(&slime_dir, &bin_dir, "slime_circle", exe_ext);
                }
            }
        } else {
            println!("cargo:warning=⚠️  未找到 nvcc，尝试从 build/ 复制 slime_circle");
            copy_from_build(&slime_dir, &bin_dir, "slime_circle", exe_ext);
        }
    } else {
        println!("cargo:warning=⚠️  未找到 slime_circle.cu，尝试从 build/ 复制");
        copy_from_build(&slime_dir, &bin_dir, "slime_circle", exe_ext);
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

fn which_cc() -> String {
    for candidate in &["gcc", "cc", "clang"] {
        if Command::new(candidate)
            .arg("--version")
            .output()
            .is_ok()
        {
            return candidate.to_string();
        }
    }
    "cc".to_string()
}

fn which_nvcc() -> Result<String, ()> {
    for candidate in &["nvcc", "/usr/local/cuda/bin/nvcc"] {
        if Command::new(candidate)
            .arg("--version")
            .output()
            .is_ok()
        {
            return Ok(candidate.to_string());
        }
    }
    Err(())
}

fn copy_from_build(slime_dir: &PathBuf, bin_dir: &PathBuf, name: &str, exe_ext: &str) {
    let build_dir = slime_dir.join("build");
    let src = build_dir.join(format!("{}{}", name, exe_ext));
    let dst = bin_dir.join(format!("{}{}", name, exe_ext));
    if src.exists() {
        match std::fs::copy(&src, &dst) {
            Ok(_) => {
                // 保留可执行权限 (Unix)
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o755));
                }
                println!("cargo:warning=📋 从 build/ 复制 {}", name);
            }
            Err(e) => {
                println!("cargo:warning=❌ 复制 {} 失败: {}", name, e);
            }
        }
    } else {
        println!("cargo:warning=❌ 找不到 {} 的预编译文件 ({}), 请先编译", name, src.display());
    }
}
