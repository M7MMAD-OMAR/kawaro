[English](mcp.md) | [العربية](mcp.ar.md)

# استخدام كوارو من تطبيق ذكاء محلي (MCP)

يستطيع كوارو العمل خادم MCP محلياً، فيقدر أي مضيف ذكاء محلي (Claude Desktop
وClaude Code وCursor وLM Studio وغيرها من المضيفات التي تشغّل خادم stdio)
أن يحوّل الملفات بكوارو. الملف لا يغادر الجهاز: لا منفذ شبكة ولا رفع ولا حساب.
المضيف يشغّل `kawaro mcp`، وكل نتيجة ملف جديد بجانب الأصل.

الخادم مطفأ افتراضياً وقراءة فقط افتراضياً.

## 1. تفعيله

حرّر `~/.config/kawaro/config.toml` على لينكس أو
`%APPDATA%\Kawaro\config.toml` على ويندوز:

```toml
[mcp]
enabled = true
read_only = false
allowed_roots = ["~/Documents", "~/Pictures", "~/Downloads"]
max_files = 50
audit = true
# audit_path = "~/logs/kawaro-audit.jsonl"
preview_px = 512
```

- `enabled`: يرفض الخادم العمل ما دامت false.
- `read_only`: عندما تكون true يعمل `kawaro_formats` و`kawaro_plan` فقط.
  اجعلها false عندما تريد التحويل.
- `allowed_roots`: لا يُقرأ ولا يُكتب إلا داخل هذه المجلدات. القائمة الفارغة
  ترفض كل مسار. `~` تعني مجلد المنزل.
- `max_files`: أكثر عدد ملفات يقبله نداء واحد.
- `audit`: سطر JSON واحد لكل نداء أداة، ويُدار عند 1 MiB إلى `.1`.
- `audit_path`: مسار تلك الأسطر. الافتراضي `mcp-audit.jsonl` داخل مجلد
  إعدادات كوارو (`~/.config/kawaro` على لينكس). المسار المطلق يُستخدم كما
  هو، و`~` و`~/...` تُوسع إلى مجلد المنزل، والمسار النسبي يُحسب من مجلد
  إعدادات كوارو. وتُنشأ المجلدات الناقصة. ويغيّر `--config` مكان قراءة
  الإعدادات لا مكان ملف التدقيق الافتراضي، لذا اضبط `audit_path` إذا
  أردت أن يتبع السجل ملف الإعدادات.
- `preview_px`: أطول جانب لمعاينة PNG في نتيجة الأداة.

## 2. إضافة المضيف

اطبع المقطع للمضيف الذي تستخدمه:

```sh
kawaro mcp --snippet claude
kawaro mcp --snippet cursor
kawaro mcp --snippet lm-studio
kawaro mcp --snippet generic
```

والمقطع هو:

```json
{"mcpServers":{"kawaro":{"command":"kawaro","args":["mcp"]}}}
```

- Claude Desktop: ضعه في `claude_desktop_config.json` (الإعدادات، المطور،
  تحرير الإعدادات) ثم أعد تشغيل التطبيق.
- Claude Code: `claude mcp add kawaro -- kawaro mcp`.
- Cursor: أضفه في `mcp.json` (الإعدادات، MCP، إضافة خادم جديد).
- LM Studio: أضفه في ملف `mcp.json` داخل التطبيق وفعّله.

المضيف يشغّل الخادم بنفسه، ولا تحتاج أيقونة النظام ولا الخدمة الخلفية لـMCP.

## 3. اطلب التحويل

مثال:

> ادمج هذه الصور الثلاث في ملف PDF واحد وضعه بجانبها.

يستدعي النموذج `kawaro_images_to_pdf`. تصل المحادثة نصاً قصيراً فيه المسار
المطلق، ومعاينة PNG صغيرة حيث يرسم المضيف الصور، ورابط `file://`، وحقولاً
منظمة. ويكون ملف PDF بجانب الصورة الأولى.

## الأدوات

| الأداة | ما تفعله |
| --- | --- |
| `kawaro_formats` | المحركات والإجراءات المتاحة على هذا الجهاز |
| `kawaro_plan` | ما سيحدث دون كتابة: الإجراءات المطابقة وأسماء المخرجات ومخطط الخيارات |
| `kawaro_convert` | تشغيل إجراء واحد بمعرّفه مع خيارات اختيارية |
| `kawaro_images_to_pdf` | دمج الصور في ملف PDF واحد |
| `kawaro_pdf` | merge وsplit وrotate وcompress وto_images |
| `kawaro_image` | resize وrotate وflip وgrayscale وcompress |

الخيارات هي نفس الخيارات المقاسة التي يعلنها ملف الإجراء، مثل
`{"quality": 60}` أو `{"dpi": 300}`.

## الأمان

- الأصل لا يُعدل أبداً. التشغيل الثاني ينشئ ملفاً جديداً مرقماً ولا يستبدل.
- لا توجد أداة حذف أو استبدال أو تعديل في المكان.
- لا يُصل إلى أي مسار إلا داخل `allowed_roots`، وتُحلّ الروابط الرمزية
  ويُرفض الهدف خارج الجذور.
- لا شيفرة شبكة في الخادم ولا مستمع HTTP.
- للإيقاف اجعل `enabled = false` أو احذف إدخال المضيف.

## الحدود

- نتائج PDF وZIP تحصل على نص ورابط بلا معاينة صورة.
- نتائج ICO وAVIF بلا معاينة.
- الخادم ينفّذ التحويل في عمليته، فلا يستخدم تقدم الصينية ولا إشعار نهاية
  العملية الخاصين بالخدمة.
- المضيفات البعيدة (مثل موصل ChatGPT على الويب) خارج النطاق، لأنها تتطلب
  مغادرة الملف للجهاز.
- يُجاب إصدار MCP 2026-07-28 ومصافحة initialize الأقدم معاً؛ ولا Tasks ولا
  MCP Apps.

## فحصه من الطرفية

```sh
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"shell"}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  | kawaro mcp
```

إذا كان الإعداد ما زال مطفأً فسيطبع الأمر
`kawaro: MCP server is disabled: set [mcp] enabled = true in config.toml`.
