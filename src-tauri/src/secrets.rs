//! انبار امن: رمزها/UUIDها/کلیدها فقط در keyring سیستم‌عامل ذخیره می‌شوند
//! (Windows Credential Manager، macOS Keychain، Linux Secret Service).
//! فایل‌های JSON برنامه هرگز رمز ندارند. رابط کاربری هم مقدار رمز را
//! پس نمی‌گیرد؛ فقط می‌تواند بنویسد، وجودش را بپرسد یا حذفش کند.
//! خواندن مقدار فقط داخل Rust و هنگام نیاز (مثلاً ساخت کانفیگ هسته) انجام می‌شود.

const SERVICE: &str = "app.netdesk.suite";

fn entry(key: &str) -> Result<keyring::Entry, String> {
    if key.is_empty() || key.len() > 200 {
        return Err("کلید نامعتبر".into());
    }
    keyring::Entry::new(SERVICE, key).map_err(|e| e.to_string())
}

pub(crate) fn read(key: &str) -> Option<String> {
    entry(key).ok()?.get_password().ok()
}

pub(crate) fn write(key: &str, value: &str) -> Result<(), String> {
    entry(key)?.set_password(value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn secret_set(key: String, value: String) -> Result<(), String> {
    write(&key, &value)
}

#[tauri::command]
pub fn secret_exists(key: String) -> bool {
    read(&key).is_some()
}

#[tauri::command]
pub fn secret_delete(key: String) -> Result<(), String> {
    match entry(&key)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
