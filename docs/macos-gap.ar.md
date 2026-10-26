[English](macos-gap.md) | [العربية](macos-gap.ar.md)

# فجوة macOS: ما الذي يفشل على `aarch64-apple-darwin`

القياس من سير عمل `macOS` (`.github/workflows/macos.yml` وADR 0009 والقصة S-069) على `macos-14` مع Rust المستقر، على مصادر 0.4.0-beta.1. التشغيلان: https://github.com/M7MMAD-OMAR/kawaro/actions/runs/40244771468 وhttps://github.com/M7MMAD-OMAR/kawaro/actions/runs/40245043420 (الثاني يضيف `cargo check --keep-going`). انتهى التشغيلان بالأخضر لأن خطوتي cargo للمعلومة فقط.

النتيجة: يفشل `cargo check --workspace --all-targets` بخطأين بالضبط، كلاهما في `crates/ui`. ويتوقف `cargo test --workspace --no-fail-fast` عند خطأ الترجمة نفسه، لذلك **لم يُشغَّل أي اختبار على macOS بعد**.

## لكل حزمة

| الحزمة | النتيجة على macOS | صنف الخطأ | القصة المقترحة |
| --- | --- | --- | --- |
| `crates/core` (`kawaro-core`) | تُترجم | لا شيء | تشغيل اختباراتها منفردة (S-069a) |
| `crates/engines` (`kawaro-engines`) | تُترجم | لا شيء (المحركات عمليات خارجية، فاختباراتها ستحتاج ffmpeg وGhostscript وغيرها على المشغّل) | S-069a ثم S-069c |
| `crates/cli` (`kawaro-cli`) | تُترجم (انتهى `Checking kawaro-cli` بلا أخطاء) | لا شيء ظاهر | S-069a |
| `crates/ui` (`kawaro-ui`) | تفشل بخطأين | بوابة `cfg` ناقصة. السطر 254 من `crates/ui/src/lib.rs` يستورد `winit::platform::startup_notify` (E0432)، وهي في winit 0.30.13 لا تُبنى إلا لـX11 وWayland. والسطر 258 يستدعي `with_activation_token` (E0599) وهي موجودة عبر تلك السمة فقط. الدالة `token_hook` مقيّدة بـ`#[cfg(unix)]`، وmacOS من عائلة Unix لكن بلا إشعار بدء التشغيل. | S-069b: تضييق البوابة إلى `cfg(all(unix, not(target_os = "macos")))` مع نسخة فارغة من `token_hook` لـmacOS |
| `crates/daemon` (`kawaro-daemon`) | لم تُبلَغ: تعتمد على `kawaro-ui` فأخطاؤها الخاصة مخفية | مجهول. تخمينات لا نتائج: استعمال الخدمة لحزمة `ashpd` (الحزمة نفسها تُترجم على macOS، لكن بوابة GlobalShortcuts لا تعمل هناك وهي مقيّدة بـ`cfg(unix)`)، واستعمال `libc`، ومسارات `/proc` وXDG | إعادة التشغيل بعد إصلاح ui في S-069b، ثم S-069d إن ظهرت أخطاء |

حقائق أخرى من السجلات:
- حزم `wayland-*` في `crates/ui` خلف `cfg(target_os = "linux")` أصلاً ولم تسبب خطأ. وتبعيات Slint بُنيت بلا مشكلة.
- `softbuffer` المضمّنة ترجمت.

## القصص المقترحة

- **S-069a** تشغيل اختبارات `core` و`engines` و`cli` على macOS لكل حزمة (`cargo test -p kawaro-core -p kawaro-engines -p kawaro-cli`) حتى لا يخفيها خطأ ui. الناتج: أول قائمة حقيقية بالاختبارات الفاشلة (المسارات و`/proc` ومجلدات XDG وسلوك الروابط والتسمية هي المشتبه بها المعتادة).
- **S-069b** إصلاح بوابة `startup_notify` في `crates/ui` (أعلاه)، ثم إعادة التشغيل لرؤية أخطاء daemon.
- **S-069c** تثبيت المحركات على المشغّل (`brew install ffmpeg ghostscript imagemagick`) لتعمل اختبارات محولات المحركات، وتحديد أين يجد macOS محركاته (مسار Homebrew `/opt/homebrew/bin` ليس في مسار بحث تطبيق رسومي افتراضياً).
- **S-069d** استبدال الافتراضات الخاصة بلينكس في daemon وcore (مجلدات الإعداد والذاكرة المؤقتة والتشغيل، ومكان مقبس IPC، والأيقونة في الشريط) بمقابلاتها في macOS، قصة لكل اكتشاف.
- **S-069e** جعل المهمة إلزامية حين تخضر، وإضافتها إلى طلبات الدمج.
- لاحقاً على جهاز Mac حقيقي (ADR 0009): تكامل Finder والتوقيع والتوثيق (notarization) وتركيز الخلية وموضعها. لا يمكن لمشغّل بلا شاشة أن يُظهر ذلك.

## لم يُتحقق منه

- كل ما يحتاج خادم نوافذ أو مدير ملفات.
- daemon واختبارات كل الحزم (مخفية بخطأ ui كما سبق).
- macOS على Intel (`macos-13` و`x86_64-apple-darwin`) لم يُجرَّب.
