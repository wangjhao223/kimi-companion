mod commands;
mod database;
mod models;
mod services;

use std::sync::{Arc, Mutex};

use tauri::Manager;

use commands::launcher::SharedInstance;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 应用级状态：当前 kimi web 实例（端口 / token / 发行版），锁内只存小对象，持锁时间极短。
    let instance: SharedInstance = Arc::new(Mutex::new(None));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(instance)
        .setup(|app| {
            // 数据库放 app_data_dir（Windows 即 %APPDATA%/<identifier>/app.db）
            let db_path = app.path().app_data_dir()?.join("app.db");
            let db = database::Database::open(&db_path)?;
            let ledger_status = services::ledger::new_shared_status();

            // 后台账本同步线程（每 30 秒一轮，首轮即全量 backfill）
            services::ledger::spawn_sync_thread(db.clone(), ledger_status.clone());
            // 联动通道：hook 写完账本后 POST 触发立即同步（只绑 loopback，失败退回轮询）
            services::ledger::spawn_trigger_listener(db.clone(), ledger_status.clone());
            // 后台 Codex 同步线程（每 15 秒一轮：Windows 侧扫本地 rollout 文件，
            // WSL 侧枚举发行版扫 \\wsl$\<distro><$HOME 解析>\.codex，各自增量入库）
            let codex_status = services::codex::new_shared_status();
            let wsl_codex_status = services::codex::new_shared_wsl_status();
            services::codex::spawn_sync_thread(
                db.clone(),
                codex_status.clone(),
                wsl_codex_status.clone(),
            );
            // 后台 Windows Kimi Code 桌面端同步线程（每 15 秒一轮，扫描 wire.jsonl 增量入库）
            let win_kimi_status = services::win_kimi::new_shared_status();
            services::win_kimi::spawn_sync_thread(db.clone(), win_kimi_status.clone());

            app.manage(db);
            app.manage(ledger_status);
            app.manage(codex_status);
            app.manage(wsl_codex_status);
            app.manage(win_kimi_status);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::launcher::list_distros,
            commands::launcher::get_launch_status,
            commands::launcher::start_kimi_web,
            commands::launcher::stop_kimi_web,
            commands::ledger::check_hook_installed,
            commands::ledger::install_hook,
            commands::ledger::get_ledger_status,
            commands::stats::get_stats_summary,
            commands::stats::get_stats_heatmap,
            commands::stats::get_stats_trend,
            commands::codex::get_codex_summary,
            commands::codex::get_codex_heatmap,
            commands::codex::get_codex_trend,
            commands::codex::get_codex_status,
            commands::wsl_codex::get_wsl_codex_summary,
            commands::wsl_codex::get_wsl_codex_heatmap,
            commands::wsl_codex::get_wsl_codex_trend,
            commands::wsl_codex::get_wsl_codex_status,
            commands::win_kimi::get_win_kimi_summary,
            commands::win_kimi::get_win_kimi_heatmap,
            commands::win_kimi::get_win_kimi_trend,
            commands::win_kimi::get_win_kimi_status,
            commands::win_kimi::get_desktop_apps_status,
            commands::win_kimi::start_desktop_app,
            commands::upgrade::get_kimi_cli_version,
            commands::upgrade::check_kimi_cli_update,
            commands::upgrade::update_kimi_cli,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
