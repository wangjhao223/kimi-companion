// 发布构建使用 windows 子系统，避免应用启动时附带控制台窗口。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    kimi_companion_lib::run()
}
