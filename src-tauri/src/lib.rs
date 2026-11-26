//! لایه‌ی Tauri: فرمان‌های IPC که رابط React صدا می‌زند.
//! منطق واقعی در crate `netcore` است؛ این‌جا فقط تبدیل به JSON، اجرای
//! غیرمسدودکننده و دسترسی به منابع سیستم (keyring، فایل، اجرای کلاینت RDP/SSH).

mod ai;
mod auto;
mod commands;
mod secrets;
mod serverkit;
mod store;
mod sysproxy;
mod tools;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .manage(tools::Running::default())
        .manage(auto::AutoState::default())
        .manage(ai::AiState::default())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            // حالت رایگان / تحلیلگر شبکه — همه محلی
            commands::dns_health,
            commands::dns_compare,
            commands::tcp_probe,
            commands::udp_dns_probe,
            commands::proxy_check,
            commands::quick_report,
            // مدیریت کانفیگ
            commands::import_profiles,
            commands::export_profiles,
            commands::list_profiles,
            commands::update_profile_meta,
            commands::delete_profile,
            // سرورها، SSH و RDP — نیازمند حساب کاربر
            commands::list_servers,
            commands::save_server,
            commands::delete_server,
            commands::server_metrics,
            commands::rdp_launch,
            // انبار امن
            secrets::secret_set,
            secrets::secret_exists,
            secrets::secret_delete,
            // پشتیبان
            store::backup_export,
            store::backup_import,
            // ابزارهای خارجی پرخطر و هسته‌ی sing-box — هر کدام با تأیید صریح
            tools::tool_list,
            tools::tool_configure,
            tools::tool_consent,
            tools::tool_forget,
            tools::tool_start,
            tools::tool_stop,
            tools::tool_log,
            tools::singbox_connect,
            tools::tool_detect,
            // اتصال خودکار و اشتراک‌ها
            auto::auto_status,
            auto::smart_connect,
            auto::auto_disconnect,
            auto::auto_settings_get,
            auto::auto_settings_set,
            auto::sub_list,
            auto::sub_add,
            auto::sub_remove,
            auto::sub_update,
            // دستیار هوش مصنوعی محلی (Ollama)
            ai::ai_status,
            ai::ai_pull,
            ai::ai_pull_status,
            ai::ai_chat,
            ai::ai_report,
            // کیت سرور برای سرور خود کاربر
            serverkit::server_kit_create,
            serverkit::server_kit_open,
        ])
        .setup(|app| {
            auto::on_start(app.handle().clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("ساخت برنامه ناموفق بود")
        .run(|app, event| {
            // هیچ پردازه‌ی فرزندی (sing-box و …) و هیچ تنظیم پروکسی‌ای بعد از بستن برنامه نمی‌ماند
            if let tauri::RunEvent::Exit = event {
                auto::disconnect_inner(app);
                app.state::<tools::Running>().kill_all();
                app.state::<ai::AiState>().kill();
            }
        });
}
