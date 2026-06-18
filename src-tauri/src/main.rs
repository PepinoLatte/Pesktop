// 发布版隐藏额外控制台窗口，避免桌面扩展启动时出现不属于产品的命令行界面
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    dasktop_lib::run()
}
