"""Writes fixtures/cases/vl_cases.jsonl and fixtures/cases/img/*.png: 24 image cases (12 ar, 12 en).
Needs Pillow with raqm for Arabic shaping. Run: python3 fixtures/make_vl_cases.py
Fixed random seed: output is reproducible."""
import json, pathlib, random
from PIL import Image, ImageDraw, ImageFont, ImageFilter

random.seed(7)
HERE = pathlib.Path(__file__).resolve().parent / "cases"
IMG = HERE / "img"; IMG.mkdir(parents=True, exist_ok=True)
FD = "/usr/share/fonts"
GF = "/home/sbarah/.local/share/fonts/google-fonts"
F = {
    "en": f"{FD}/liberation-sans-fonts/LiberationSans-Regular.ttf",
    "enb": f"{FD}/liberation-sans-fonts/LiberationSans-Bold.ttf",
    "ar": f"{FD}/google-noto/NotoNaskhArabic-Regular.ttf",
    "arb": f"{FD}/google-noto/NotoNaskhArabic-Bold.ttf",
    "en_hand": f"{GF}/Caveat/Caveat-VariableFont_wght.ttf",
    "ar_hand": f"{GF}/Aref Ruqaa/ArefRuqaa-Regular.ttf",
}
def font(key, size):
    return ImageFont.truetype(F[key], size, layout_engine=ImageFont.Layout.RAQM)

CASES = []
def case(lang, kind, img, prompt, gold, check):
    name = f"{kind}_{lang}.png"
    img.convert("RGB").save(IMG / name)
    CASES.append({"id": f"v{len(CASES)+1:02d}_{kind}_{lang}", "lang": lang, "kind": kind,
                  "image": f"img/{name}", "prompt": prompt, "gold": gold, "check": check})

def T(d, xy, s, f, fill=(20, 20, 20), anchor=None, ar=False):
    d.text(xy, s, font=f, fill=fill, anchor=anchor or ("ra" if ar else "la"),
           direction="rtl" if ar else None, language="ar" if ar else None)

def mid(d, y, s, f, W, fill=(20, 20, 20), ar=False):
    T(d, (W // 2, y), s, f, fill, "ma", ar)

# ---------- receipt (2 per language) ----------
RECEIPTS = {
    "ar": [("سوبرماركت الواحة", "2025-08-17", [("خبز", "3.00"), ("حليب", "9.50"), ("بيض", "14.25")], "26.75"),
           ("مخبز الريحان", "2025-02-03", [("كعك", "6.00"), ("شاي", "4.50")], "10.50")],
    "en": [("Northside Grocers", "2025-08-17", [("Bread", "3.00"), ("Milk", "9.50"), ("Eggs", "14.25")], "26.75"),
           ("Cedar Bakery", "2025-02-03", [("Pretzel", "6.00"), ("Tea", "4.50")], "10.50")],
}
def receipt(lang, i):
    ar = lang == "ar"; store, date, items, total = RECEIPTS[lang][i]
    W, H = 520, 150 + 46 * len(items) + 110; im = Image.new("RGB", (W, H), (250, 250, 246)); d = ImageDraw.Draw(im)
    fb, fr = font("arb" if ar else "enb", 34), font("ar" if ar else "en", 28)
    mid(d, 24, store, fb, W, ar=ar)
    mid(d, 76, date, font("en", 28), W)
    d.line([(30, 125), (W - 30, 125)], fill=(120, 120, 120), width=2)
    y = 145
    for n, p in items:
        if ar: T(d, (W - 36, y), n, fr, ar=True); T(d, (36, y), p, fr, anchor="la")
        else: T(d, (36, y), n, fr); T(d, (W - 36, y), p, fr, anchor="ra")
        y += 46
    d.line([(30, y + 6), (W - 30, y + 6)], fill=(120, 120, 120), width=2)
    lab = "المجموع" if ar else "TOTAL"
    if ar: T(d, (W - 36, y + 24), lab, fb, ar=True); T(d, (36, y + 24), total, fb, anchor="la")
    else: T(d, (36, y + 24), lab, fb); T(d, (W - 36, y + 24), total, fb, anchor="ra")
    prompt = ("اقرأ الإيصال وأعد كائن JSON فقط بالمفاتيح store و date و total و items_count. التاريخ بصيغة YYYY-MM-DD." if ar else
              "Read the receipt and reply with one JSON object only, with the keys store, date, total, items_count. Date as YYYY-MM-DD.")
    case(lang, f"receipt{i+1}", im, prompt, {"fields": {"store": store, "date": date, "total": float(total), "items_count": len(items)}}, "json_fields")
for L in ("ar", "en"):
    for i in (0, 1): receipt(L, i)

# ---------- table to markdown ----------
def table(lang):
    ar = lang == "ar"
    head = ["المنتج", "الكمية", "السعر", "المجموع"] if ar else ["Item", "Qty", "Price", "Total"]
    rows = ([["قلم", "10", "2.50", "25.00"], ["دفتر", "4", "6.00", "24.00"], ["مسطرة", "3", "1.75", "5.25"]] if ar else
            [["Pen", "10", "2.50", "25.00"], ["Notebook", "4", "6.00", "24.00"], ["Ruler", "3", "1.75", "5.25"]])
    W, H = 760, 330; im = Image.new("RGB", (W, H), "white"); d = ImageDraw.Draw(im)
    f, fb = font("ar" if ar else "en", 30), font("arb" if ar else "enb", 30)
    cw, rh, x0, y0 = 170, 62, 40, 30
    for r in range(4):
        for c in range(4):
            x = x0 + ((3 - c) if ar else c) * cw; y = y0 + r * rh
            d.rectangle([x, y, x + cw, y + rh], outline=(60, 60, 60), width=2, fill=(228, 232, 240) if r == 0 else None)
            txt = (head if r == 0 else rows[r - 1])[c]
            col = (3 - c) if ar else c  # logical column c sits at the right in Arabic
            xx = x0 + col * cw
            T(d, (xx + cw // 2, y + 14), txt, fb if r == 0 else f, anchor="ma", ar=ar)
    prompt = ("حوّل الجدول إلى جدول Markdown. رتب الأعمدة من اليمين إلى اليسار كما تُقرأ، ولا تضف شيئا آخر." if ar else
              "Convert the table to a Markdown table. Add nothing else.")
    case(lang, "table", im, prompt, {"rows": [head] + rows}, "table")
table("ar"); table("en")

# ---------- bar charts (2 per language) ----------
def chart(lang, i):
    ar = lang == "ar"
    data = ([[("يناير", 31), ("فبراير", 58), ("مارس", 44), ("أبريل", 23), ("مايو", 37)],
             [("الشمال", 120), ("الجنوب", 85), ("الشرق", 140), ("الغرب", 95)]] if ar else
            [[("Jan", 31), ("Feb", 58), ("Mar", 44), ("Apr", 23), ("May", 37)],
             [("North", 120), ("South", 85), ("East", 140), ("West", 95)]])[i]
    W, H = 760, 460; im = Image.new("RGB", (W, H), "white"); d = ImageDraw.Draw(im)
    f = font("ar" if ar else "en", 26); mx = max(v for _, v in data)
    n = len(data); bw = 80; gap = (W - 120 - n * bw) // (n + 1); base = H - 70
    cols = [(70, 120, 200), (230, 140, 50), (80, 160, 100), (180, 70, 90), (120, 90, 170)]
    d.line([(60, 30), (60, base), (W - 40, base)], fill=(40, 40, 40), width=3)
    seq = list(reversed(data)) if ar else data
    for k, (lab, v) in enumerate(seq):
        x = 70 + gap + k * (bw + gap); h = int((base - 70) * v / mx)
        d.rectangle([x, base - h, x + bw, base], fill=cols[k % 5])
        T(d, (x + bw // 2, base - h - 34), str(v), f, anchor="ma")
        T(d, (x + bw // 2, base + 10), lab, f, anchor="ma", ar=ar)
    best = max(data, key=lambda t: t[1])
    prompt = ("ما هو اسم العمود الأعلى في الرسم البياني وما قيمته؟ أجب بالاسم والقيمة فقط." if ar else
              "Which bar is the largest in the chart, and what is its value? Answer with the label and the value only.")
    case(lang, f"chart{i+1}", im, prompt, {"all": [[best[0]], ["#" + str(best[1])]]}, "keywords")
for L in ("ar", "en"):
    for i in (0, 1): chart(L, i)

# ---------- shape scene with a question ----------
def scene(lang):
    ar = lang == "ar"
    W, H = 760, 480; im = Image.new("RGB", (W, H), (150, 205, 245)); d = ImageDraw.Draw(im)
    d.rectangle([0, 340, W, H], fill=(110, 180, 90))
    d.ellipse([620, 30, 710, 120], fill=(250, 215, 60))
    for cx, cy in ((150, 90), (400, 60)):
        for dx, dy, r in ((0, 0, 34), (36, 8, 28), (-36, 8, 26)): d.ellipse([cx + dx - r, cy + dy - r, cx + dx + r, cy + dy + r], fill="white")
    roof = (200, 50, 50) if not ar else (50, 90, 200)
    d.rectangle([260, 230, 460, 370], fill=(235, 220, 180)); d.polygon([(240, 235), (360, 140), (480, 235)], fill=roof)
    d.rectangle([335, 290, 385, 370], fill=(110, 70, 40)); d.rectangle([280, 255, 320, 295], fill=(170, 220, 250))
    trees = [60, 530, 640] if not ar else [70, 560]
    for tx in trees:
        d.rectangle([tx - 10, 300, tx + 10, 380], fill=(110, 70, 40)); d.ellipse([tx - 45, 220, tx + 45, 320], fill=(40, 130, 60))
    n = len(trees)
    if ar:
        prompt = "ما لون سقف البيت وكم شجرة في الصورة؟"
        gold = {"all": [["أزرق", "ازرق"], ["#2", "شجرتان", "شجرتين", "اثنتان", "اثنتين"]]}
    else:
        prompt = "What colour is the roof of the house and how many trees are in the picture?"
        gold = {"all": [["red"], ["#3", "three"]]}
    case(lang, "scene", im, prompt, gold, "keywords")
scene("ar"); scene("en")

# ---------- UI screenshot ----------
def ui(lang):
    ar = lang == "ar"
    W, H = 760, 420; im = Image.new("RGB", (W, H), (225, 228, 235)); d = ImageDraw.Draw(im)
    d.rectangle([60, 40, W - 60, H - 40], fill="white", outline=(150, 150, 150), width=2)
    d.rectangle([60, 40, W - 60, 90], fill=(50, 60, 90))
    f, fb = font("ar" if ar else "en", 28), font("arb" if ar else "enb", 28)
    T(d, ((W - 80, 52) if ar else (80, 52)), "حفظ التغييرات" if ar else "Save changes", fb, (255, 255, 255), ar=ar)
    T(d, ((W - 80, 120) if ar else (80, 120)), "هل تريد المتابعة؟ لا يمكن التراجع." if ar else "Do you want to continue? This cannot be undone.", f, ar=ar)
    labels = [("إلغاء", "حذف", "حفظ"), ("Cancel", "Delete", "Save")][0 if ar else 1]
    colors = [(200, 200, 205), (205, 45, 45), (40, 100, 220)]
    xs = [W - 60 - 40 - 150 * (k + 1) - 20 * k for k in range(3)] if ar else [100 + 170 * k for k in range(3)]
    order = labels if not ar else labels  # first label is rightmost for Arabic
    for k in range(3):
        x = xs[k]; d.rounded_rectangle([x, 260, x + 150, 322], radius=10, fill=colors[k])
        T(d, (x + 75, 276), labels[k], fb, (255, 255, 255) if k else (40, 40, 40), anchor="ma", ar=ar)
    prompt = "ما النص المكتوب على الزر الأحمر؟" if ar else "What does the red button say?"
    case(lang, "ui", im, prompt, {"all": [["حذف"] if ar else ["delete"]]}, "keywords")
ui("ar"); ui("en")

# ---------- form fields ----------
def form(lang):
    ar = lang == "ar"
    rows = ([("الاسم الكامل", "هدى الخطيب"), ("تاريخ الميلاد", "1990-04-12"), ("رقم الهاتف", "0551234567"), ("البلد", "الأردن")] if ar else
            [("Full name", "Sara Collins"), ("Date of birth", "1990-04-12"), ("Phone", "0551234567"), ("Country", "Canada")])
    keys = ["name", "dob", "phone", "country"]
    W, H = 760, 420; im = Image.new("RGB", (W, H), (247, 247, 250)); d = ImageDraw.Draw(im)
    f, fb = font("ar" if ar else "en", 28), font("arb" if ar else "enb", 28)
    mid(d, 16, "نموذج التسجيل" if ar else "Registration form", fb, W, ar=ar)
    y = 85
    for lab, val in rows:
        if ar:
            T(d, (W - 40, y), lab, fb, ar=True); d.rectangle([40, y - 6, 480, y + 52], fill="white", outline=(120, 120, 120), width=2)
            T(d, (470, y), val, f if not val[0].isdigit() else font("en", 28), ar=ar, anchor="ra")
        else:
            T(d, (40, y), lab, fb); d.rectangle([280, y - 6, W - 40, y + 52], fill="white", outline=(120, 120, 120), width=2)
            T(d, (292, y), val, f)
        y += 74
    prompt = ("استخرج بيانات النموذج كـ JSON فقط بالمفاتيح name و dob و phone و country. dob بصيغة YYYY-MM-DD." if ar else
              "Extract the form data as one JSON object with the keys name, dob, phone, country. dob as YYYY-MM-DD.")
    case(lang, "form", im, prompt, {"fields": {k: v for k, (_, v) in zip(keys, rows)}}, "json_fields")
form("ar"); form("en")

# ---------- document with stamps (no ID, just reference number) ----------
def stamp_layer(W, H, text, size, color, angle, circle, pos, ar):
    L = Image.new("RGBA", (W, H), (0, 0, 0, 0)); d = ImageDraw.Draw(L)
    f = font("arb" if ar else "enb", size); tb = d.textbbox((0, 0), text, font=f, direction="rtl" if ar else None, language="ar" if ar else None)
    w, h = tb[2] - tb[0], tb[3] - tb[1]; pad = 22
    S = Image.new("RGBA", (w + 2 * pad + 20, h + 2 * pad + 40), (0, 0, 0, 0)); sd = ImageDraw.Draw(S)
    sd.rounded_rectangle([0, 0, S.width - 1, S.height - 1], radius=18 if not circle else S.height // 2, outline=color + (230,), width=6)
    sd.text((S.width // 2, S.height // 2), text, font=f, fill=color + (230,), anchor="mm", direction="rtl" if ar else None, language="ar" if ar else None)
    S = S.rotate(angle, expand=True, resample=Image.BICUBIC); L.paste(S, pos, S); return L
def stamps(lang):
    ar = lang == "ar"
    W, H = 760, 560; im = Image.new("RGB", (W, H), (252, 250, 244)); d = ImageDraw.Draw(im)
    f, fb = font("ar" if ar else "en", 27), font("arb" if ar else "enb", 30)
    T(d, ((W - 50, 40) if ar else (50, 40)), "رقم المرجع: 7305" if ar else "Reference No: 7305", fb, ar=ar)
    body = (["نؤكد استلام الدفعة الأولى عن عقد التوريد.", "سيتم إرسال الإيصال النهائي خلال خمسة أيام عمل.", "شكرا لتعاونكم معنا."] if ar else
            ["We confirm receipt of the first payment for the supply contract.", "The final receipt will be sent within five working days.", "Thank you for your cooperation."])
    for k, s in enumerate(body): T(d, ((W - 50, 110 + k * 46) if ar else (50, 110 + k * 46)), s, f, ar=ar)
    layers = [("مدفوع" if ar else "PAID", 56, (200, 40, 40), 14, True, (60, 290)),
              ("تم الاستلام" if ar else "RECEIVED 2025-06-03", 30, (40, 70, 170), -8, False, (330, 350))]
    base = im.convert("RGBA")
    for t, sz, col, ang, circ, pos in layers: base = Image.alpha_composite(base, stamp_layer(W, H, t, sz, col, ang, circ, pos, ar))
    im = base
    prompt = ("ماذا يقول الختمان في الوثيقة وما رقم المرجع؟" if ar else "What do the two stamps on the document say, and what is the reference number?")
    gold = {"all": [["مدفوع"], ["الاستلام", "استلام"], ["#7305"]]} if ar else {"all": [["paid"], ["received"], ["#7305"]]}
    case(lang, "stamps", im, prompt, gold, "keywords")
stamps("ar"); stamps("en")

# ---------- handwriting-like ----------
def hand(lang):
    ar = lang == "ar"
    W, H = 760, 380; im = Image.new("RGB", (W, H), (250, 246, 225)); d = ImageDraw.Draw(im)
    for y in range(80, H, 70): d.line([(30, y), (W - 30, y)], fill=(190, 205, 230), width=2)
    lines = (["اتصل بسامر عند الساعة 6 مساء.", "أحضر الملف الأخضر ونسختين."] if ar else ["Call Dana at 6 pm.", "Bring the blue folder and two copies."])
    f = font("ar_hand" if ar else "en_hand", 46)
    for k, s in enumerate(lines):
        L = Image.new("RGBA", (W, 90), (0, 0, 0, 0)); ld = ImageDraw.Draw(L)
        ld.text(((W - 50) if ar else 50, 10), s, font=f, fill=(25, 35, 120, 255), anchor="ra" if ar else "la", direction="rtl" if ar else None, language="ar" if ar else None)
        L = L.rotate(random.uniform(-2.5, 2.5), resample=Image.BICUBIC); im.paste(L, (0, 40 + k * 90), L)
    prompt = "انسخ النص المكتوب بخط اليد كما هو." if ar else "Transcribe the handwritten note exactly."
    gold = {"all": [["سامر"], ["#6"], ["الأخضر"], ["نسختين", "نسختان"]]} if ar else {"all": [["dana"], ["#6"], ["blue"], ["folder"]]}
    case(lang, "handwriting", im, prompt, gold, "keywords")
hand("ar"); hand("en")

# ---------- low contrast ----------
def lowc(lang):
    ar = lang == "ar"
    W, H = 760, 260; im = Image.new("RGB", (W, H), (214, 214, 214)); d = ImageDraw.Draw(im)
    f = font("ar" if ar else "en", 44)
    ls = ["رمز الاستلام 5827", "البوابة ب، الطابق الأول"] if ar else ["Pickup code: 5827", "Gate B, first floor"]
    for k, s in enumerate(ls): T(d, ((W - 50, 50 + k * 80) if ar else (50, 50 + k * 80)), s, f, (186, 186, 186), ar=ar)
    prompt = "اكتب النص الموجود في الصورة (التباين ضعيف)." if ar else "Write the text in the image (the contrast is low)."
    gold = {"all": [["#5827"], ["البوابة"]]} if ar else {"all": [["#5827"], ["gate b"]]}
    case(lang, "lowcontrast", im, prompt, gold, "keywords")
lowc("ar"); lowc("en")

# ---------- rotated text ----------
def rot(lang):
    ar = lang == "ar"
    W, H = 760, 300; im = Image.new("RGB", (W, H), "white"); d = ImageDraw.Draw(im)
    f = font("ar" if ar else "en", 40)
    ls = ["المستودع 12 يغلق الساعة 5 مساء", "يوم الجمعة."] if ar else ["Warehouse 12 closes at 5 PM", "on Friday."]
    for k, s in enumerate(ls): T(d, ((W - 40, 60 + k * 70) if ar else (40, 60 + k * 70)), s, f, ar=ar)
    im = im.rotate(-90, expand=True)
    prompt = "الصورة مقلوبة جانبا. اكتب النص الموجود فيها." if ar else "The image is rotated sideways. Write the text it contains."
    gold = {"all": [["المستودع"], ["#12"], ["#5"], ["الجمعة"]]} if ar else {"all": [["warehouse"], ["#12"], ["#5"], ["friday"]]}
    case(lang, "rotated", im, prompt, gold, "keywords")
rot("ar"); rot("en")

assert len(CASES) == 24 and sum(c["lang"] == "ar" for c in CASES) == 12
with open(HERE / "vl_cases.jsonl", "w", encoding="utf-8") as fh:
    for c in CASES: fh.write(json.dumps(c, ensure_ascii=False) + "\n")
print("wrote", len(CASES))
