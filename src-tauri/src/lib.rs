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

            app.manage(db);
            app.manage(ledger_status);
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
            commands::stats::get_quota,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
