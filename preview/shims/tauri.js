// شبیه‌ساز Tauri برای پیش‌نمایش در مرورگر (api.ts در این حالت isMock=true است)
export const invoke = () => Promise.reject(new Error("خارج از Tauri"));
export const save = async () => null;
export const writeTextFile = async () => {};
export const isPermissionGranted = async () => false;
export const requestPermission = async () => "denied";
export const sendNotification = () => {};
export const open = async () => null;
export const ask = async (m) => window.confirm(m);
