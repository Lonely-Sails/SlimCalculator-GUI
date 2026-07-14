# 🐌 史莱姆区块扫描器 (SlimCalculator-GUI)

为 [minelogy-dev/slime-calculator](https://github.com/minelogy-dev/slime-calculator) 设计的桌面图形化界面，基于 **Tauri v2** 构建。

![Tauri](https://img.shields.io/badge/Tauri-v2-FFC131?logo=tauri)
![Rust](https://img.shields.io/badge/Rust-2021-dea584?logo=rust)
![License](https://img.shields.io/badge/License-Apache%202.0-blue)

## ✨ 功能

- 🖥️ **桌面 GUI** — 简洁直观的参数配置和结果显示
- 🚀 **完整流水线** — 一键运行 `slime_main` → `slime_circle` → `slime_cmp`
- 📊 **可视化图表** — 散点图展示结果分布，颜色标识密度
- 📋 **详情表格** — 支持排序查看和 CSV 导出
- 🎯 **灵活配置** — 支持所有原始参数，可选流水线环节

## 📦 下载

从 [Releases](https://github.com/jobber/SlimCalculator-GUI/releases) 页面下载对应平台的最新版本：

| 平台 | 格式 |
|------|------|
| macOS (Intel) | `.dmg` |
| macOS (Apple Silicon) | `.dmg` |
| Windows | `.msi` / `.exe` |
| Linux | `.AppImage` / `.deb` |

## 🔧 开发环境

### 前置要求

- **Rust** (推荐使用 [rustup](https://rustup.rs/))
- **Node.js** 18+ (推荐使用 [bun](https://bun.sh/))
- macOS: Xcode Command Line Tools
- Linux: WebKit2GTK 等 (`sudo apt install libwebkit2gtk-4.1-dev`)

### 本地运行

```bash
# 1. 克隆仓库
git clone https://github.com/jobber/SlimCalculator-GUI.git
cd SlimCalculator-GUI

# 2. 安装前端依赖
bun install

# 3. 启动开发模式（需要先编译 CUDA 二进制到 build/）
bun tauri dev
```

### 构建发布版

```bash
bun tauri build
```

构建产物在 `src-tauri/target/release/bundle/` 目录下。
CUDA 二进制文件已内嵌在 `src-tauri/bin/`，开箱即用。

## 🎮 使用说明

1. 打开应用，输入 Minecraft 世界种子和搜索参数
2. 可选：开启圆形筛选和/或距离筛选
3. 点击「开始扫描」等待结果
4. 在结果面板查看图表和表格，可导出为 CSV

> **注意**: CUDA 程序运行时需要 NVIDIA GPU，大型搜索范围可能需要数分钟。

## 🔄 流水线说明

```
slime_main  (矩形扫描)  →  candidates.csv
   ↓ (可选)
slime_circle (圆形筛选)  →  circles.csv
   ↓ (可选)
slime_cmp   (距离筛选)   →  result.csv
```

详细参数说明见 [slime-calculator/README.md](slime-calculator/README.md)

## 📄 License

Apache License 2.0
