# vendor — سورس ابزارهای دیگران

کد این پوشه **کار NetDesk نیست**. هر زیرپوشه متعلق به سازنده‌ی خودش است و فقط تحت لایسنس خودش (فایل `LICENSE` همان پوشه) منتشر می‌شود. لایسنس MIT خود NetDesk شامل این پوشه نمی‌شود. این کدها داخل NetDesk لینک یا همراه فایل نصبی آن بسته‌بندی نمی‌شوند. NetDesk فقط برنامه‌ای را که کاربر جداگانه ساخته، به‌صورت یک پردازه‌ی مستقل اجرا می‌کند.

این کپی‌ها گرفته شدند تا کاربر بتواند طبق [آموزش‌ها](../docs/tutorials/start.fa.md) هر ابزار را از همین مخزن بسازد.

تاریخ کپی: ۲۰۲۶-۱۱-۲۶

| پوشه | سازنده | لایسنس | منبع | commit |
|---|---|---|---|---|
| `sing-box` | nekohasekai / SagerNet | GPL-3.0-or-later (با شرط نام در LICENSE) | https://github.com/SagerNet/sing-box | `491b845be7ac4bdaa3e3f9443505eb57e061a28c` |
| `Xray-core` | Project X / XTLS | MPL-2.0 | https://github.com/XTLS/Xray-core | `b56ffef338c8096c7dbe78eaa76f1cb05a328d6f` |
| `MasterHttpRelayVPN` | Amin Mahmoudi (masterking32) | MIT | https://github.com/masterking32/MasterHttpRelayVPN | `0a019e172472a9bb1829ca32639c824e77decf8c` |
| `mhr-cfw` | denuitt1 | MIT | https://github.com/denuitt1/mhr-cfw | `44b2481040e17d5f450c2c65d6f2fa52bb7e1584` |
| `FlowDriver` | NullLatency | MIT | https://github.com/NullLatency/FlowDriver | `fa8ee8950107ee1bde58d3adf6e479e7bd2fcd73` |
| `SNI-Spoofing` | patterniha | GPL-3.0 | https://github.com/patterniha/SNI-Spoofing | `13b78cf7e073f38d9cadcff542faf4a00b0a6de2` |
| `sni-spoofing-rust` | therealaleph (روش از patterniha) | MIT | https://github.com/therealaleph/sni-spoofing-rust | `d2956025c31d96f0f0a341af4f1a8eda204857c7` |
| `SNI-Spoofing-Go` | aleskxyz (روش از patterniha) | GPL-3.0 | https://github.com/aleskxyz/SNI-Spoofing-Go | `f60734498c5dcdfd85705f4a682af5871d01895c` |

## تغییرات نسبت به منبع

هیچ فایل کدی ویرایش نشده است. فقط این موارد کپی **نشده‌اند**:

- پوشه‌های `.github` و فایل‌های `.gitmodules` (پس زیرماژول‌های sing-box هم همراه نیستند)
- فایل‌های اجرایی، کتابخانه‌ای، درایور و فشرده (`exe`، `dll`، `sys`، `so`، `dylib`، `zip`، `gz`، `apk`، `jar`)، از جمله درایورهای WinDivert در sing-box
- پوشه‌ی `releases` در sni-spoofing-rust

دلیل: فایل‌های باینری بررسی نشده‌اند و نباید از این مخزن توزیع شوند.

## مهم

این کدها **ممیزی امنیتی نشده‌اند**. بعضی از آن‌ها گواهی CA ریشه نصب می‌کنند یا دسترسی مدیر سیستم می‌خواهند. پیش از ساختن یا اجرای هر کدام، آموزش و هشدارهایش را بخوانید.

اگر سازنده‌ی یکی از این پروژه‌ها هستید و می‌خواهید کپی آن از این‌جا حذف شود، یک issue باز کنید.
