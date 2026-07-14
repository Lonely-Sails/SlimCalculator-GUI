const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ── DOM References ────────────────────────────────────
const $ = (s) => document.querySelector(s);
const $$ = (s) => document.querySelectorAll(s);

const dom = {
  seed: $("#seed"),
  startX: $("#startX"),
  startZ: $("#startZ"),
  endX: $("#endX"),
  endZ: $("#endZ"),
  sizeX: $("#sizeX"),
  sizeZ: $("#sizeZ"),
  threshold: $("#threshold"),
  enableCircle: $("#enableCircle"),
  circleParams: $("#circleParams"),
  circleRadius: $("#circleRadius"),
  enableCmp: $("#enableCmp"),
  cmpParams: $("#cmpParams"),
  cmpDistance: $("#cmpDistance"),
  cmpThreshold: $("#cmpThreshold"),

  runBtn: $("#runBtn"),
  stopBtn: $("#stopBtn"),
  binStatus: $("#bin-status"),

  progressContainer: $("#progressContainer"),
  progressFill: $("#progressFill"),
  progressMsg: $("#progressMsg"),

  emptyState: $("#emptyState"),
  resultsOverview: $("#resultsOverview"),
  chartContainer: $("#chartContainer"),
  tableContainer: $("#tableContainer"),

  statTotal: $("#statTotal"),
  statAvg: $("#statAvg"),
  statMax: $("#statMax"),
  statDist: $("#statDist"),

  resultChart: $("#resultChart"),
  resultBody: $("#resultBody"),
  rowCount: $("#rowCount"),
  exportBtn: $("#exportBtn"),
  distHeader: $("#distHeader"),

  toastContainer: $("#toastContainer"),
};

// ── State ────────────────────────────────────────────
let records = [];
let isRunning = false;
let unlisten = null;

// ── Toast ────────────────────────────────────────────
function showToast(msg, type = "info") {
  const el = document.createElement("div");
  el.className = `toast ${type}`;
  el.textContent = msg;
  dom.toastContainer.appendChild(el);
  setTimeout(() => el.remove(), 3500);
}

// ── Event Bus (Tauri events) ────────────────────────
async function setupEventListeners() {
  if (unlisten) return;
  unlisten = await listen("pipeline-progress", (event) => {
    const { step, message, percent } = event.payload;

    dom.progressContainer.style.display = "block";
    dom.progressFill.style.width = `${percent}%`;
    dom.progressMsg.textContent = message;
  });
}

// ── Check Binaries ──────────────────────────────────
async function checkBinaries() {
  try {
    const status = await invoke("get_binaries_status");
    const allOk = status.slime_main && status.slime_circle && status.slime_cmp;
    const anyOk = status.slime_main || status.slime_circle || status.slime_cmp;

    if (allOk) {
      dom.binStatus.textContent = "✅ 二进制文件就绪";
      dom.binStatus.className = "badge ok";
      dom.runBtn.disabled = false;
    } else if (anyOk) {
      const parts = [];
      if (!status.slime_main) parts.push("slime_main");
      if (!status.slime_circle) parts.push("slime_circle");
      if (!status.slime_cmp) parts.push("slime_cmp");
      dom.binStatus.textContent = `⚠️ 缺少: ${parts.join(", ")}`;
      dom.binStatus.className = "badge warn";
      dom.runBtn.disabled = false; // Allow running with what's available
    } else {
      dom.binStatus.textContent = "❌ 未找到二进制文件 - 请先编译";
      dom.binStatus.className = "badge error";
      dom.binStatus.title = `查找路径: ${status.bin_dir}`;
      dom.runBtn.disabled = true;
    }
  } catch (e) {
    dom.binStatus.textContent = "❌ 检查失败";
    dom.binStatus.className = "badge error";
    console.error(e);
  }
}

// ── Read Inputs ─────────────────────────────────────
function getParams() {
  return {
    seed: parseInt(dom.seed.value) || 0,
    start_x: parseInt(dom.startX.value) || 0,
    start_z: parseInt(dom.startZ.value) || 0,
    end_x: parseInt(dom.endX.value) || 0,
    end_z: parseInt(dom.endZ.value) || 0,
    size_x: parseInt(dom.sizeX.value) || 1,
    size_z: parseInt(dom.sizeZ.value) || 1,
    threshold: parseInt(dom.threshold.value) || 1,
    circle_radius: dom.enableCircle.checked
      ? parseInt(dom.circleRadius.value) || 5
      : null,
    cmp_distance: dom.enableCmp.checked
      ? parseInt(dom.cmpDistance.value) || 1000
      : null,
    cmp_threshold: dom.enableCmp.checked
      ? parseInt(dom.cmpThreshold.value) || 1
      : null,
  };
}

// ── Run Pipeline ────────────────────────────────────
async function runScan() {
  if (isRunning) return;
  isRunning = true;

  const params = getParams();

  // Validate
  if (params.start_x > params.end_x || params.start_z > params.end_z) {
    showToast("起始坐标不能大于结束坐标", "error");
    isRunning = false;
    return;
  }

  // UI state
  dom.runBtn.style.display = "none";
  dom.stopBtn.style.display = "flex";
  dom.emptyState.style.display = "none";
  dom.resultsOverview.style.display = "none";
  dom.chartContainer.style.display = "none";
  dom.tableContainer.style.display = "none";
  dom.progressContainer.style.display = "block";
  dom.progressFill.style.width = "0%";
  dom.progressMsg.textContent = "🚀 正在启动...";

  try {
    await setupEventListeners();

    const startTime = performance.now();
    records = await invoke("run_pipeline", { params });
    const elapsed = ((performance.now() - startTime) / 1000).toFixed(1);

    if (records.length === 0) {
      showToast(`扫描完成，未找到符合条件的区域 (${elapsed}s)`, "info");
      dom.progressMsg.textContent = "✅ 扫描完成，未找到结果";
      dom.emptyState.style.display = "flex";
      dom.emptyState.querySelector("h2").textContent = "😕 没有找到结果";
      dom.emptyState.querySelector("p").textContent =
        "尝试扩大搜索范围或降低阈值";
    } else {
      showToast(
        `🎉 找到 ${records.length} 个区域 (${elapsed}s)`,
        "success"
      );
      dom.progressMsg.textContent = `✅ 扫描完成！找到 ${records.length} 个区域`;
      renderResults(records);
    }
  } catch (e) {
    showToast(`❌ ${e}`, "error");
    dom.progressMsg.textContent = `❌ 出错: ${e}`;
    dom.emptyState.style.display = "flex";
    dom.emptyState.querySelector("h2").textContent = "😵 扫描出错";
    dom.emptyState.querySelector("p").textContent =
      typeof e === "string" ? e : "请检查参数和控制台输出";
    console.error(e);
  }

  dom.runBtn.style.display = "flex";
  dom.stopBtn.style.display = "none";
  isRunning = false;
}

// ── Render Results ──────────────────────────────────
function renderResults(data) {
  records = data;

  // Overview stats
  dom.statTotal.textContent = records.length;
  const avg =
    records.reduce((s, r) => s + r.slime_count, 0) / records.length;
  const max = Math.max(...records.map((r) => r.slime_count));
  dom.statAvg.textContent = avg.toFixed(1);
  dom.statMax.textContent = max;

  const hasDist = records.some((r) => r.distance != null);
  if (hasDist) {
    const minDist = Math.min(
      ...records.map((r) => r.distance).filter((d) => d != null)
    );
    dom.statDist.textContent = minDist.toFixed(1);
    dom.distHeader.style.display = "";
  } else {
    dom.statDist.textContent = "-";
    dom.distHeader.style.display = "none";
  }

  dom.resultsOverview.style.display = "block";

  // Chart
  renderChart(records);

  // Table
  const hasDistance = records.some((r) => r.distance != null);
  dom.resultBody.innerHTML = records
    .map(
      (r) => `<tr>
        <td>${r.x}</td>
        <td>${r.z}</td>
        <td><span class="badge" style="background:${slimeColor(
          r.slime_count,
          max
        )}20;color:${slimeColor(r.slime_count, max)};border:1px solid ${
        slimeColor(r.slime_count, max)
      }40">${r.slime_count}</span></td>
        ${
          hasDistance
            ? `<td>${r.distance != null ? r.distance.toFixed(1) : "-"}</td>`
            : ""
        }
      </tr>`
    )
    .join("");

  dom.rowCount.textContent = `${records.length} 条记录`;
  dom.tableContainer.style.display = "block";
}

function slimeColor(count, max) {
  const ratio = count / max;
  if (ratio > 0.7) return "#f85149";
  if (ratio > 0.4) return "#d29922";
  return "#7ee787";
}

// ── Chart (Scatter Plot) ────────────────────────────
function renderChart(data) {
  dom.chartContainer.style.display = "block";

  const canvas = dom.resultChart;
  const rect = canvas.parentElement.getBoundingClientRect();
  const size = Math.min(rect.width - 24, 600);
  const dpr = 2; // retina
  canvas.width = size * dpr;
  canvas.height = size * dpr;
  canvas.style.width = `${size}px`;
  canvas.style.height = `${size}px`;

  const ctx = canvas.getContext("2d");
  const W = canvas.width;
  const H = canvas.height;
  const pad = 70;
  const tickSize = 6;

  const xs = data.map((r) => r.x);
  const zs = data.map((r) => r.z);
  const counts = data.map((r) => r.slime_count || 1);
  const minX = Math.min(...xs);
  const maxX = Math.max(...xs);
  const minZ = Math.min(...zs);
  const maxZ = Math.max(...zs);

  const rangeX = Math.max(maxX - minX, 1);
  const rangeZ = Math.max(maxZ - minZ, 1);
  const maxCount = Math.max(...counts);

  // Compute plot area: use the larger range to determine scale so both axes use the same scale
  const maxRange = Math.max(rangeX, rangeZ);
  const availW = W - pad * 2;
  const availH = H - pad * 2;
  const plotSize = Math.min(availW, availH);
  const scale = plotSize / maxRange;

  const plotW = rangeX * scale;
  const plotH = rangeZ * scale;
  const ox = (W - plotW) / 2;
  const oy = (H - plotH) / 2;

  const fontSize = (pt) => `${Math.round(pt * dpr)}px -apple-system, sans-serif`;

  ctx.clearRect(0, 0, W, H);

  // Background
  ctx.fillStyle = "#0d1117";
  ctx.fillRect(0, 0, W, H);

  // ── Grid ──
  ctx.strokeStyle = "#21262d";
  ctx.lineWidth = 1;
  for (let i = 0; i <= 4; i++) {
    const t = i / 4;
    const x = ox + plotW * t;
    const y = oy + plotH * t;
    // Vertical
    ctx.beginPath();
    ctx.moveTo(x, oy);
    ctx.lineTo(x, oy + plotH);
    ctx.stroke();
    // Horizontal
    ctx.beginPath();
    ctx.moveTo(ox, y);
    ctx.lineTo(ox + plotW, y);
    ctx.stroke();
  }

  // ── Tick marks ──
  ctx.strokeStyle = "#8b949e";
  ctx.lineWidth = 1;
  for (let i = 0; i <= 4; i++) {
    const t = i / 4;
    const x = ox + plotW * t;
    const y = oy + plotH * t;
    // X ticks (bottom)
    ctx.beginPath();
    ctx.moveTo(x, oy + plotH);
    ctx.lineTo(x, oy + plotH + tickSize);
    ctx.stroke();
    // Z ticks (left)
    ctx.beginPath();
    ctx.moveTo(ox, y);
    ctx.lineTo(ox - tickSize, y);
    ctx.stroke();
  }

  // ── Axis tick labels ──
  ctx.fillStyle = "#8b949e";
  ctx.font = fontSize(11);
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  for (let i = 0; i <= 4; i++) {
    const t = i / 4;
    const val = minX + rangeX * t;
    const x = ox + plotW * t;
    ctx.fillText(Math.round(val), x, oy + plotH + tickSize + 4);
  }

  ctx.textAlign = "right";
  ctx.textBaseline = "middle";
  for (let i = 0; i <= 4; i++) {
    const t = i / 4;
    const zval = minZ + rangeZ * t;
    const y = oy + plotH * t;
    ctx.fillText(Math.round(zval), ox - tickSize - 4, y);
  }

  // ── Axis titles ──
  ctx.fillStyle = "#6e7681";
  ctx.font = fontSize(13);
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  ctx.fillText("X →", ox + plotW / 2, oy + plotH + tickSize + 22);

  ctx.save();
  ctx.textAlign = "center";
  ctx.textBaseline = "bottom";
  ctx.translate(ox - tickSize - 24, oy + plotH / 2);
  ctx.rotate(-Math.PI / 2);
  ctx.fillText("Z →", 0, 0);
  ctx.restore();

  // ── Data points ──
  const radius = Math.max(4, Math.min(10, (plotSize / Math.max(data.length, 1)) * 1.5));

  // Sort by count so higher count draws on top
  const sorted = [...data].sort((a, b) => a.slime_count - b.slime_count);

  for (const r of sorted) {
    const x = ox + (r.x - minX) * scale;
    const z = oy + (r.z - minZ) * scale;

    const ratio = maxCount > 0 ? r.slime_count / maxCount : 0;
    const rSize = radius * (0.5 + ratio * 0.5);

    ctx.beginPath();
    ctx.arc(x, z, rSize, 0, Math.PI * 2);

    if (ratio > 0.7) {
      ctx.fillStyle = "rgba(248, 81, 73, 0.8)";
    } else if (ratio > 0.4) {
      ctx.fillStyle = "rgba(210, 153, 34, 0.8)";
    } else {
      ctx.fillStyle = "rgba(126, 231, 135, 0.8)";
    }
    ctx.fill();

    ctx.strokeStyle = "rgba(255,255,255,0.2)";
    ctx.lineWidth = 1;
    ctx.stroke();
  }

  // ── Title ──
  ctx.fillStyle = "#8b949e";
  ctx.font = fontSize(13);
  ctx.textAlign = "left";
  ctx.textBaseline = "top";
  ctx.fillText(`共 ${data.length} 个结果`, pad, 16);
}

// ── Export CSV ─────────────────────────────────────
async function exportCSV() {
  if (records.length === 0) {
    showToast("没有数据可导出", "info");
    return;
  }

  // Browser download approach (works in both dev and production)
  const csv = csvString(records);
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `slime_results_${Date.now()}.csv`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
  showToast("✅ CSV 文件已下载", "success");
}

function csvString(data) {
  const hasDist = data.some((r) => r.distance != null);
  let csv = hasDist ? "x,z,slime_count,distance\n" : "x,z,slime_count\n";
  for (const r of data) {
    if (hasDist) {
      csv += `${r.x},${r.z},${r.slime_count},${r.distance != null ? r.distance.toFixed(6) : ""}\n`;
    } else {
      csv += `${r.x},${r.z},${r.slime_count}\n`;
    }
  }
  return csv;
}

// ── UI Event Handlers ──────────────────────────────
function setupUI() {
  // Toggle sub-params
  dom.enableCircle.addEventListener("change", () => {
    dom.circleParams.style.display = dom.enableCircle.checked ? "block" : "none";
  });

  dom.enableCmp.addEventListener("change", () => {
    dom.cmpParams.style.display = dom.enableCmp.checked ? "block" : "none";
  });

  // Run/Stop
  dom.runBtn.addEventListener("click", runScan);
  dom.stopBtn.addEventListener("click", () => {
    // Tauri doesn't easily kill subprocesses, but we can stop listening
    showToast("无法中断正在执行的 CUDA 程序，请等待完成", "info");
  });

  // Export
  dom.exportBtn.addEventListener("click", exportCSV);

  // Enter key to run
  const inputs = document.querySelectorAll("input");
  inputs.forEach((inp) => {
    inp.addEventListener("keydown", (e) => {
      if (e.key === "Enter") runScan();
    });
  });

  // Window resize -> re-render chart
  let resizeTimer;
  window.addEventListener("resize", () => {
    clearTimeout(resizeTimer);
    resizeTimer = setTimeout(() => {
      if (records.length > 0) renderChart(records);
    }, 300);
  });
}

// ── Init ────────────────────────────────────────────
window.addEventListener("DOMContentLoaded", async () => {
  setupUI();
  await checkBinaries();
});
