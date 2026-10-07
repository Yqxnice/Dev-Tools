//! 下载任务控制：暂停 / 继续 / 取消的全局状态管理 + 共享下载主循环。
//!
//! 下载循环通过 `Arc<DownloadControl>` 读取原子标志决定是否暂停或取消；
//! 前端通过 invoke `pause_download` / `resume_download` / `cancel_download` 命令修改标志。
//!
//! 暂停实现为"停止读取流 → flush 文件 → 记录已下载字节数 → 返回 Paused"，
//! 继续时发起带 `Range: bytes={offset}-` 的新请求追加写入（CDN 均支持 Range）。

use crate::logger;
use crate::types::DownloadProgress;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Window};
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use std::sync::LazyLock;
use futures_util::StreamExt;

/// 下载任务控制状态：通过 Arc 共享给下载循环和控制命令
pub struct DownloadControl {
    paused: AtomicBool,
    cancelled: AtomicBool,
}

impl DownloadControl {
    pub fn new() -> Self {
        Self {
            paused: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::Relaxed);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::Relaxed);
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

impl Default for DownloadControl {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局下载任务注册表：task_id -> Arc<DownloadControl>
static DOWNLOAD_TASKS: LazyLock<Mutex<HashMap<String, Arc<DownloadControl>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 注册下载任务，返回 RAII 守卫（`Guard` 持有控制句柄并在 drop 时自动清理注册表）。
///
/// **拒绝重复注册**：原实现直接 `map.insert` 覆盖同 `task_id`，会导致
/// ① 旧任务与注册表脱钩 → 暂停/取消指令全部作用到新任务，旧任务在
/// `while ctrl.is_paused()` 中**无限自旋无法终止**；
/// ② 两个任务写同一 `file_path` 交叉损坏文件；
/// ③ 旧任务结束时 `remove_task` 把新任务从表里删掉，新任务此后不可控。
pub fn register_task(task_id: &str) -> Result<TaskGuard, String> {
    let ctrl = Arc::new(DownloadControl::new());
    let mut map = DOWNLOAD_TASKS
        .lock()
        .map_err(|e| format!("任务表锁中毒: {}", e))?;
    if map.contains_key(task_id) {
        return Err(format!("下载任务已在进行中: {}", task_id));
    }
    map.insert(task_id.to_string(), ctrl.clone());
    Ok(TaskGuard {
        task_id: task_id.to_string(),
        ctrl,
    })
}

/// 任务注册表条目的 RAII 守卫。
///
/// 下载 future 若被 drop（invoke 取消、任务 panic、应用关闭），原实现会让
/// 注册表项永久残留 —— 残留项又会让后续同名下载撞上重复注册。
/// 守卫保证"作用域结束即移除"，且按**实例身份**移除，绝不会误删别人的条目。
pub struct TaskGuard {
    task_id: String,
    ctrl: Arc<DownloadControl>,
}

impl std::fmt::Debug for TaskGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskGuard")
            .field("task_id", &self.task_id)
            .finish_non_exhaustive()
    }
}

impl TaskGuard {
    /// 下载循环使用的控制句柄
    pub fn ctrl(&self) -> &Arc<DownloadControl> {
        &self.ctrl
    }

    /// 任务 id
    pub fn task_id(&self) -> &str {
        &self.task_id
    }
}

impl Drop for TaskGuard {
    fn drop(&mut self) {
        let _ = remove_task_if_same(&self.task_id, &self.ctrl);
    }
}

/// 仅当注册表中该 id 仍指向**同一个**控制实例时才移除。
///
/// 显式 `remove_task(id)` 之后若又注册了同名任务，drop 守卫时
/// `Arc::ptr_eq` 会失败，从而不会把新任务误删。
pub fn remove_task_if_same(task_id: &str, ctrl: &Arc<DownloadControl>) -> Result<(), String> {
    let mut map = DOWNLOAD_TASKS
        .lock()
        .map_err(|e| format!("任务表锁中毒: {}", e))?;
    if map
        .get(task_id)
        .map(|c| Arc::ptr_eq(c, ctrl))
        .unwrap_or(false)
    {
        map.remove(task_id);
    }
    Ok(())
}

/// 移除下载任务（下载完成或取消后调用）
pub fn remove_task(task_id: &str) -> Result<(), String> {
    let mut map = DOWNLOAD_TASKS.lock().map_err(|e| format!("任务表锁中毒: {}", e))?;
    map.remove(task_id);
    Ok(())
}

/// 暂停下载任务
pub fn pause_task(task_id: &str) -> Result<(), String> {
    let map = DOWNLOAD_TASKS.lock().map_err(|e| e.to_string())?;
    match map.get(task_id) {
        Some(ctrl) => {
            ctrl.pause();
            Ok(())
        }
        None => Err(format!("下载任务不存在: {}", task_id)),
    }
}

/// 继续下载任务
pub fn resume_task(task_id: &str) -> Result<(), String> {
    let map = DOWNLOAD_TASKS.lock().map_err(|e| e.to_string())?;
    match map.get(task_id) {
        Some(ctrl) => {
            ctrl.resume();
            Ok(())
        }
        None => Err(format!("下载任务不存在: {}", task_id)),
    }
}

/// 取消下载任务
pub fn cancel_task(task_id: &str) -> Result<(), String> {
    let map = DOWNLOAD_TASKS.lock().map_err(|e| e.to_string())?;
    match map.get(task_id) {
        Some(ctrl) => {
            ctrl.cancel();
            Ok(())
        }
        None => Err(format!("下载任务不存在: {}", task_id)),
    }
}

/// 下载单次尝试的结果
enum DownloadOutcome {
    /// 下载完成：实际字节数、声明总大小
    Completed { actual_size: u64, total_size: u64 },
    /// 已暂停：已下载字节数、声明总大小
    Paused { downloaded: u64, total_size: u64 },
    /// 已取消
    Cancelled,
    /// 错误：错误信息、本次已下载字节数、本次声明总大小
    /// 携带 downloaded/total_size 让调用方正确更新外层偏移，避免 resume 返回 200（截断文件）
    /// 后重试仍用陈旧偏移导致文件损坏
    Error(String, u64, u64),
}

/// 发送下载进度事件
#[allow(clippy::too_many_arguments)]
fn emit_progress(
    window: &Window,
    task_id: &str,
    version: &str,
    downloaded: u64,
    total: u64,
    status: &str,
    completed: bool,
    success: bool,
    paused: bool,
) {
    let percentage = if total > 0 {
        let pct = (downloaded as f64 / total as f64) * 100.0;
        if completed {
            pct.min(100.0)
        } else {
            pct.min(99.9)
        }
    } else {
        0.0
    };
    let progress = DownloadProgress {
        task_id: task_id.to_string(),
        version: version.to_string(),
        downloaded,
        total,
        percentage,
        status: status.to_string(),
        completed,
        success,
        paused,
    };
    let _ = window.emit("download_progress", &progress);
}

/// 发送下载完成终态事件
pub fn emit_completed(
    window: &Window,
    task_id: &str,
    version: &str,
    actual_size: u64,
    total: u64,
) {
    let total = if total > 0 { total } else { actual_size };
    emit_progress(
        window, task_id, version, actual_size, total, "下载完成", true, true, false,
    );
}

/// 发送下载失败终态事件
pub fn emit_failed(
    window: &Window,
    task_id: &str,
    version: &str,
    downloaded: u64,
    total: u64,
) {
    emit_progress(
        window, task_id, version, downloaded, total, "下载失败", true, false, false,
    );
}

/// 计算文件 SHA256（下载完成后调用，非增量式，支持暂停/继续后的完整校验）
pub async fn compute_file_hash(file_path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut file = tokio::fs::File::open(file_path)
        .await
        .map_err(|e| format!("打开文件失败: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024]; // 256KB 缓冲区
    loop {
        let n = file
            .read(&mut buf)
            .await
            .map_err(|e| format!("读取文件失败: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// 计算文件 MD5（MySQL 官方仅提供 MD5 校验和）
pub async fn compute_file_md5(file_path: &Path) -> Result<String, String> {
    use md5::{Digest, Md5};
    let mut file = tokio::fs::File::open(file_path)
        .await
        .map_err(|e| format!("打开文件失败: {}", e))?;
    let mut hasher = Md5::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .await
            .map_err(|e| format!("读取文件失败: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// 单次下载尝试：流式写入，进度事件按时间节流（≥200ms 且 ≥64KB），循环检查暂停/取消标志。
///
/// - `start_offset > 0` 时发起 Range 请求并追加写入（CDN 支持 206 Partial Content）
/// - 暂停时 flush 文件并返回 `Paused`，调用方等待恢复后再次调用以续传
/// - 取消时立即返回 `Cancelled`，调用方删除部分文件
/// - `known_total` 为前次已知总大小：续传时若服务器未返回 Content-Length
///   （如 chunked 编码或 200 响应无长度头），回退到前次值，避免前端百分比突然归零
#[allow(clippy::too_many_arguments)]
async fn download_once(
    window: &Window,
    http_client: &reqwest::Client,
    url: &str,
    file_path: &Path,
    version_label: &str,
    task_id: &str,
    start_offset: u64,
    known_total: u64,
    ctrl: &DownloadControl,
) -> DownloadOutcome {
    // 构造请求：续传时带 Range 头
    let req = if start_offset > 0 {
        http_client
            .get(url)
            .header("Range", format!("bytes={}-", start_offset))
    } else {
        http_client.get(url)
    };

    let response = match req.send().await {
        Ok(r) => r,
        Err(e) => return DownloadOutcome::Error(format!("下载请求失败: {}", e), start_offset, known_total),
    };

    if !response.status().is_success() {
        return DownloadOutcome::Error(format!("下载失败: HTTP {}", response.status()), start_offset, known_total);
    }

    // 206 Partial Content 表示服务器支持续传；200 表示服务器返回完整内容（不支持 Range 或新请求）
    let is_partial = response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    // 续传时若本响应无 Content-Length，回退到 known_total，避免前端百分比突然归零
    let total_size = if is_partial {
        response
            .content_length()
            .map(|c| c + start_offset)
            .unwrap_or(known_total)
    } else {
        response.content_length().unwrap_or(known_total)
    };

    // 打开文件：续传追加，否则新建/截断
    let mut file = if is_partial {
        match tokio::fs::OpenOptions::new()
            .append(true)
            .open(file_path)
            .await
        {
            Ok(f) => f,
            Err(e) => return DownloadOutcome::Error(format!("打开文件失败: {}", e), start_offset, known_total),
        }
    } else {
        match tokio::fs::File::create(file_path).await {
            Ok(f) => f,
            Err(e) => return DownloadOutcome::Error(format!("创建文件失败: {}", e), start_offset, known_total),
        }
    };

    let mut stream = response.bytes_stream();
    let mut downloaded = if is_partial { start_offset } else { 0u64 };
    let mut last_emitted = downloaded;
    // 时间节流：高速链路下按字节节流(1MB)仍会产生上百事件/秒，淹没 WebView 主线程导致 UI 卡死。
    // 改为 ≥200ms 且 ≥64KB 才上报一次，事件频率上限恒定 ~5/秒；终态事件不受影响。
    let mut last_emit_time = Instant::now();

    loop {
        // 检查暂停：flush 文件后返回，让调用方等待恢复
        if ctrl.is_paused() {
            if let Err(e) = file.flush().await {
                return DownloadOutcome::Error(format!("刷新文件失败: {}", e), downloaded, total_size);
            }
            return DownloadOutcome::Paused {
                downloaded,
                total_size,
            };
        }
        // 检查取消：立即返回，调用方删除文件
        if ctrl.is_cancelled() {
            return DownloadOutcome::Cancelled;
        }

        match stream.next().await {
            Some(Ok(chunk)) => {
                if let Err(e) = file.write_all(&chunk).await {
                    return DownloadOutcome::Error(format!("写入文件失败: {}", e), downloaded, total_size);
                }
                downloaded += chunk.len() as u64;

                // 时间节流进度事件：≥200ms 且 ≥64KB，避免高频 IPC 冻结 UI
                if downloaded - last_emitted >= 64 * 1024
                    && last_emit_time.elapsed() >= Duration::from_millis(200)
                {
                    last_emitted = downloaded;
                    last_emit_time = Instant::now();
                    emit_progress(
                        window,
                        task_id,
                        version_label,
                        downloaded,
                        total_size,
                        "下载中",
                        false,
                        false,
                        false,
                    );
                }
            }
            Some(Err(e)) => {
                return DownloadOutcome::Error(format!("下载数据中断: {}", e), downloaded, total_size);
            }
            None => {
                // 流结束
                if let Err(e) = file.flush().await {
                    return DownloadOutcome::Error(format!("刷新文件失败: {}", e), downloaded, total_size);
                }
                return DownloadOutcome::Completed {
                    actual_size: downloaded,
                    total_size,
                };
            }
        }
    }
}

/// 下载主流程：注册任务 → 循环调用 download_once → 处理暂停/继续/取消/重试 → 返回字节数。
///
/// 成功返回 `(实际字节数, 声明总大小)`，调用方自行计算 SHA256 并校验后发送终态事件。
/// 失败/取消时本函数已发送终态事件（已取消/下载失败）。
#[allow(clippy::too_many_arguments)]
pub async fn download_file(
    app_handle: &AppHandle,
    window: &Window,
    task_id: &str,
    url: &str,
    file_path: &Path,
    version_label: &str,
    http_client: &reqwest::Client,
    max_retries: u32,
) -> Result<(u64, u64), String> {
    // 守卫持有注册表条目：正常 return / 未来被 drop / panic 都会自动清理
    let guard = register_task(task_id)?;
    let ctrl = guard.ctrl().clone();
    let mut downloaded: u64 = 0;
    let mut total_size: u64 = 0;
    let mut error_count = 0u32;

    loop {
        // 循环入口检查取消（暂停后被取消的情况）
        if ctrl.is_cancelled() {
            let _ = tokio::fs::remove_file(file_path).await;
            let _ = remove_task(task_id);
            emit_progress(
                window, task_id, version_label, downloaded, total_size, "已取消", true, false, false,
            );
            return Err("下载已取消".to_string());
        }

        match download_once(
            window,
            http_client,
            url,
            file_path,
            version_label,
            task_id,
            downloaded,
            total_size,
            &ctrl,
        )
        .await
        {
            DownloadOutcome::Completed {
                actual_size,
                total_size: ts,
            } => {
                // 续传场景下本次响应可能无 Content-Length，保留前次已知 total_size
                let final_total = if ts > 0 { ts } else { total_size };
                let _ = remove_task(task_id);
                return Ok((actual_size, final_total));
            }
            DownloadOutcome::Paused {
                downloaded: dl,
                total_size: ts,
            } => {
                downloaded = dl;
                // 仅在拿到新值时更新 total_size，避免 chunked 编码响应把它归零
                if ts > 0 {
                    total_size = ts;
                }
                // 发送已暂停事件，前端显示继续/取消按钮
                emit_progress(
                    window, task_id, version_label, downloaded, total_size, "已暂停", false, false, true,
                );
                // 等待恢复或取消（不持有 HTTP 连接，避免超时）
                while ctrl.is_paused() && !ctrl.is_cancelled() {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                if ctrl.is_cancelled() {
                    let _ = tokio::fs::remove_file(file_path).await;
                    let _ = remove_task(task_id);
                    emit_progress(
                        window, task_id, version_label, downloaded, total_size, "已取消", true, false, false,
                    );
                    return Err("下载已取消".to_string());
                }
                // 已恢复 → 循环继续，download_once 将带 Range 续传
                error_count = 0;
            }
            DownloadOutcome::Cancelled => {
                let _ = tokio::fs::remove_file(file_path).await;
                let _ = remove_task(task_id);
                emit_progress(
                    window, task_id, version_label, downloaded, total_size, "已取消", true, false, false,
                );
                return Err("下载已取消".to_string());
            }
            DownloadOutcome::Error(e, dl, ts) => {
                // 用本次实际下载字节数更新外层偏移，避免 resume 返回 200（截断文件）后
                // 重试仍用陈旧偏移导致文件损坏（详见 DownloadOutcome::Error 注释）
                downloaded = dl;
                if ts > 0 {
                    total_size = ts;
                }
                error_count += 1;
                logger::warn(
                    app_handle,
                    &format!("下载失败({}/{}): {}", error_count, max_retries, e),
                );
                // 下载途中用户取消
                if ctrl.is_cancelled() {
                    let _ = tokio::fs::remove_file(file_path).await;
                    let _ = remove_task(task_id);
                    emit_progress(
                        window, task_id, version_label, downloaded, total_size, "已取消", true, false, false,
                    );
                    return Err("下载已取消".to_string());
                }
                if error_count >= max_retries {
                    let _ = tokio::fs::remove_file(file_path).await;
                    let _ = remove_task(task_id);
                    emit_failed(window, task_id, version_label, downloaded, total_size);
                    return Err(format!("下载失败（已重试 {} 次）: {}", max_retries, e));
                }
                // 等待后重试（保留 downloaded 偏移以续传）
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────────
// 公共路径工具：被 Python/JetBrains fetcher 和 lib.rs 命令共用
// ──────────────────────────────────────────────────────────────────

/// DevTools 下载根目录（安装包保存位置）：
/// 优先系统下载目录/DevTools，回退到 系统缓存目录/DevTools/downloads
pub fn devtools_download_dir() -> PathBuf {
    dirs::download_dir()
        .map(|d| d.join("DevTools"))
        .or_else(|| dirs::cache_dir().map(|d| d.join("DevTools").join("downloads")))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// 将配置文件备份到下载目录下按版本号命名的子文件夹中。
///
/// 备份路径：`{下载根目录}/{version}/{原文件名}.{unix秒}.bak`
/// 用于密码重置等高风险操作前对核心配置（my.ini / pg_hba.conf 等）留底。
///
/// 返回备份文件路径；若源文件不存在则返回 None（不报错，不影响主流程）。
pub async fn backup_config_to_download_dir(
    source: &Path,
    version: &str,
) -> Option<PathBuf> {
    if !source.exists() {
        return None;
    }
    let dir = devtools_download_dir().join(version);
    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        eprintln!("[config-backup] 创建备份目录失败 {:?}: {}", dir, e);
        return None;
    }
    let fname = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "config".to_string());
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dest = dir.join(format!("{}.{}.bak", fname, ts));
    match tokio::fs::copy(source, &dest).await {
        Ok(_) => Some(dest),
        Err(e) => {
            eprintln!("[config-backup] 备份配置文件 {:?} 失败: {}", source, e);
            None
        }
    }
}

/// DevTools 日志导出目录
pub fn devtools_log_dir() -> PathBuf {
    dirs::download_dir()
        .map(|d| d.join("DevTools").join("logs"))
        .or_else(|| dirs::cache_dir().map(|d| d.join("DevTools").join("logs")))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// DevTools 环境变量备份目录
pub fn devtools_env_backup_dir() -> PathBuf {
    dirs::download_dir()
        .map(|d| d.join("DevTools").join("env-backups"))
        .or_else(|| dirs::cache_dir().map(|d| d.join("DevTools").join("env-backups")))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// DevTools 配置导出目录（导入/导出 JSON 文件存放位置）
pub fn devtools_config_dir() -> PathBuf {
    dirs::download_dir()
        .map(|d| d.join("DevTools").join("config"))
        .or_else(|| dirs::cache_dir().map(|d| d.join("DevTools").join("config")))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 注册表是全局 static，测试之间用互不冲突的 id 避免互相干扰
    const A: &str = "__test_reg_a__";
    const B: &str = "__test_reg_b__";
    const C: &str = "__test_reg_c__";

    #[test]
    fn register_task_rejects_duplicate_id() {
        let guard = register_task(A).expect("首次注册应成功");
        let err = register_task(A).expect_err("重复注册应被拒绝");
        assert!(err.contains("已在进行中"), "错误信息应说明已存在: {}", err);
        // 首次注册的条目必须还在（不能被覆盖成第二个实例）
        let map = DOWNLOAD_TASKS.lock().unwrap();
        let stored = map.get(A).expect("首次注册的条目应仍在");
        assert!(Arc::ptr_eq(stored, guard.ctrl()), "不得被覆盖");
        drop(map);
        drop(guard);
    }

    #[test]
    fn guard_drop_removes_entry() {
        {
            let guard = register_task(B).expect("注册应成功");
            assert!(DOWNLOAD_TASKS.lock().unwrap().contains_key(B));
            drop(guard);
        }
        assert!(
            !DOWNLOAD_TASKS.lock().unwrap().contains_key(B),
            "守卫 drop 后注册表项必须被移除"
        );
    }

    #[test]
    fn remove_task_if_same_ignores_foreign_instance() {
        let guard = register_task(C).expect("注册应成功");
        let foreign = Arc::new(DownloadControl::new());
        remove_task_if_same(C, &foreign).expect("移除操作本身不应报错");
        assert!(
            DOWNLOAD_TASKS.lock().unwrap().contains_key(C),
            "身份不匹配时绝不能删除条目"
        );
        remove_task_if_same(C, guard.ctrl()).expect("身份匹配时应移除");
        assert!(!DOWNLOAD_TASKS.lock().unwrap().contains_key(C));
        drop(guard);
    }

    #[test]
    fn pause_resume_cancel_toggle_flags() {
        let c = DownloadControl::new();
        assert!(!c.is_paused() && !c.is_cancelled());
        c.pause();
        assert!(c.is_paused());
        c.resume();
        assert!(!c.is_paused());
        c.cancel();
        assert!(c.is_cancelled());
    }

    #[test]
    fn pause_resume_cancel_on_missing_task_returns_error() {
        let missing = "__test_missing_task__";
        let _ = remove_task(missing);
        assert!(pause_task(missing).is_err());
        assert!(resume_task(missing).is_err());
        assert!(cancel_task(missing).is_err());
    }

    #[test]
    fn control_on_registered_task_is_reachable_via_registry() {
        let guard = register_task("__test_reach__").expect("注册应成功");
        guard.ctrl().pause();
        assert!(pause_task("__test_reach__").is_ok());
        assert!(DOWNLOAD_TASKS
            .lock()
            .unwrap()
            .get("__test_reach__")
            .expect("条目应在")
            .is_paused());
        drop(guard);
    }
}
