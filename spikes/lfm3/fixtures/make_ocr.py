#!/usr/bin/env python3
"""Generate the OCR fixtures: ocr/<name>.png plus ocr/<name>.txt ground truth.

Usage: python3 -I spikes/lfm3/fixtures/make_ocr.py

Needs Pillow built with raqm (shaping and bidi) and ImageMagick `magick` for the
skew and noise pass. All text is original, with no personal data. The .txt files
hold the logical-order text, one line per rendered line, which is what a perfect
OCR would return. For two-column pages the right (Arabic) or left (English)
column comes first; for tables each row is its cells joined by one space in
reading order.
"""
import pathlib
import subprocess

from PIL import Image, ImageDraw, ImageFont

OUT = pathlib.Path(__file__).resolve().parent / "ocr"
OUT.mkdir(exist_ok=True)

FONT_DIR = "/usr/share/fonts"
EN = f"{FONT_DIR}/liberation-sans-fonts/LiberationSans-Regular.ttf"
EN_BOLD = f"{FONT_DIR}/liberation-sans-fonts/LiberationSans-Bold.ttf"
EN_SERIF = f"{FONT_DIR}/google-noto/NotoSerif-Regular.ttf"
AR_AMIRI = f"{FONT_DIR}/amiri-fonts/Amiri-Regular.ttf"
AR_AMIRI_BOLD = f"{FONT_DIR}/amiri-fonts/Amiri-Bold.ttf"
AR_NASKH = f"{FONT_DIR}/google-noto/NotoNaskhArabic-Regular.ttf"
AR_NASKH_BOLD = f"{FONT_DIR}/google-noto/NotoNaskhArabic-Bold.ttf"

PAPER = (252, 252, 250)
INK = (20, 20, 20)


def font(path, size):
    return ImageFont.truetype(path, size, layout_engine=ImageFont.Layout.RAQM)


def write_txt(name, lines):
    (OUT / f"{name}.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")


def text_ar(d, xy, s, f, anchor="ra", fill=INK):
    d.text(xy, s, font=f, fill=fill, anchor=anchor, direction="rtl", language="ar")


def text_en(d, xy, s, f, anchor="la", fill=INK):
    d.text(xy, s, font=f, fill=fill, anchor=anchor, language="en")


def save(img, name):
    img.save(OUT / f"{name}.png", optimize=True)


def page(width, height, bg=PAPER):
    img = Image.new("RGB", (width, height), bg)
    return img, ImageDraw.Draw(img)


def photo_effect(name, rotate="2.5", seed="7"):
    """Skew, grey surround, sensor noise and a little blur, like a phone photo."""
    src = OUT / f"{name}.png"
    subprocess.run(
        [
            "magick", str(src),
            "-background", "#6b675f", "-rotate", rotate,
            "-seed", seed, "-attenuate", "0.55", "+noise", "Gaussian",
            "-blur", "0x0.9", "-brightness-contrast", "-6x-4",
            "-quality", "90", str(src),
        ],
        check=True,
    )


# ---------------------------------------------------------------- English

def en_clean():
    lines = [
        "Notice of Lift Maintenance",
        "",
        "Dear residents,",
        "",
        "The lift in Building B will be serviced on Tuesday, 14 October 2025.",
        "Work starts at 8:00 AM and finishes at 4:00 PM on each of three days.",
        "Please use the stairs during this period. We are sorry for any",
        "inconvenience this may cause.",
        "",
        "For questions, call the building office on 555-0142 between 9:00 AM",
        "and 5:00 PM, Monday to Friday.",
        "",
        "Thank you for your cooperation.",
        "Building Management",
    ]
    f, fb = font(EN, 30), font(EN_BOLD, 38)
    img, d = page(1240, 780)
    y = 70
    for i, s in enumerate(lines):
        if s:
            text_en(d, (90, y), s, fb if i == 0 else f)
        y += 48
    save(img, "en1_clean_letter")
    write_txt("en1_clean_letter", [s for s in lines if s])


def en_small():
    lines = [
        "Annual Members Meeting: Notice",
        "The board invites all members to the annual meeting at 6:30 PM on",
        "Thursday, 23 November 2025, in the conference room on the third floor.",
        "Agenda:",
        "1. Read and approve the minutes of the previous meeting.",
        "2. Present the financial report for the year ended 31 December 2024.",
        "3. Discuss next year's budget and the membership fee.",
        "4. Elect two new members of the board.",
        "5. Any other business.",
        "Please bring your membership card. If you cannot attend, you may",
        "appoint another member to vote for you by writing to the secretary",
        "at least 3 days before the meeting. Light refreshments will be served",
        "after the meeting. Parking is free for members in the north lot.",
        "Doors open at 6:00 PM and the meeting starts promptly at 6:30 PM.",
        "Minutes will be posted on the notice board within 10 days.",
    ]
    f, fb = font(EN, 17), font(EN_BOLD, 19)
    img, d = page(760, 430)
    y = 28
    for i, s in enumerate(lines):
        text_en(d, (30, y), s, fb if i == 0 else f)
        y += 25
    save(img, "en2_small_font")
    write_txt("en2_small_font", lines)


def en_columns():
    left = [
        "Library Hours",
        "The library is open from",
        "Saturday to Thursday, from",
        "9:00 AM to 8:00 PM, and is",
        "closed on Friday. You can",
        "borrow 5 books for 14 days.",
        "Each loan can be renewed",
        "once.",
    ]
    right = [
        "Borrowing Rules",
        "Bring your library card each",
        "time you borrow. Please keep",
        "quiet in the reading rooms.",
        "Food and drinks are not",
        "allowed in the halls.",
        "For help, call 555-0188 or",
        "ask at the front desk.",
    ]
    f, fb = font(EN_SERIF, 27), font(EN_BOLD, 32)
    img, d = page(1100, 520)
    for x, col in ((70, left), (590, right)):
        y = 60
        for i, s in enumerate(col):
            text_en(d, (x, y), s, fb if i == 0 else f)
            y += 50
    d.line([(550, 50), (550, 470)], fill=(160, 160, 160), width=2)
    save(img, "en3_two_columns")
    write_txt("en3_two_columns", left + right)


def en_table():
    title = "Invoice 20457"
    date = "Date: 5/11/2025"
    head = ["Item", "Qty", "Price", "Total"]
    rows = [
        ["Notebook", "4", "12", "48"],
        ["Ink pen", "10", "3", "30"],
        ["Plastic folder", "6", "5", "30"],
        ["Office bag", "1", "85", "85"],
        ["Printer paper", "5", "22", "110"],
    ]
    total = ["Total", "", "", "303"]
    f, fb = font(EN, 28), font(EN_BOLD, 28)
    img, d = page(1000, 640)
    text_en(d, (60, 40), title, font(EN_BOLD, 36))
    text_en(d, (60, 92), date, f)
    xs = [60, 520, 650, 800]
    right_edge = [None, 590, 740, 940]
    y = 160
    d.line([(50, y - 8), (950, y - 8)], fill=INK, width=2)
    for r, row in enumerate([head] + rows + [total]):
        for c, cell in enumerate(row):
            if not cell:
                continue
            fn = fb if r in (0, len(rows) + 1) else f
            if c == 0:
                text_en(d, (xs[0], y), cell, fn)
            else:
                text_en(d, (right_edge[c], y), cell, fn, anchor="ra")
        y += 56
        d.line([(50, y - 8), (950, y - 8)], fill=(150, 150, 150), width=1)
    save(img, "en4_table_list")
    out = [title, date, " ".join(head)]
    out += [" ".join(r) for r in rows]
    out += ["Total 303"]
    write_txt("en4_table_list", out)


def en_noisy():
    lines = [
        "RECEIPT",
        "Receipt no: 88213",
        "Date: 12/11/2025",
        "Received 250 for maintenance services",
        "Payment method: cash",
        "Received by: accounts office",
        "The receiver keeps a copy of this receipt",
        "Thank you for your business",
    ]
    f, fb = font(EN, 34), font(EN_BOLD, 44)
    img, d = page(900, 560, bg=(238, 234, 224))
    y = 50
    for i, s in enumerate(lines):
        text_en(d, (60, y), s, fb if i == 0 else f)
        y += 62
    save(img, "en5_noisy_skewed")
    write_txt("en5_noisy_skewed", lines)
    photo_effect("en5_noisy_skewed", rotate="-3", seed="11")


# ----------------------------------------------------------------- Arabic

def ar_clean_diacritics():
    lines = [
        "إِشْعَارٌ بِمَوْعِدِ الصِّيَانَةِ",
        "",
        "تَحِيَّةً طَيِّبَةً وَبَعْدُ،",
        "",
        "سَتَبْدَأُ أَعْمَالُ صِيَانَةِ المِصْعَدِ يَوْمَ الثُّلَاثَاءِ 14 تِشْرِينَ الأَوَّلِ،",
        "وَتَسْتَمِرُّ ثَلَاثَةَ أَيَّامٍ مِنَ السَّاعَةِ 8 صَبَاحًا إِلَى 4 عَصْرًا.",
        "نَرْجُو اسْتِخْدَامَ الدَّرَجِ خِلَالَ هَذِهِ الْمُدَّةِ، وَنَعْتَذِرُ عَنْ أَيِّ إِزْعَاجٍ.",
        "",
        "لِلِاسْتِفْسَارِ يُرْجَى الِاتِّصَالُ بِالْمَكْتَبِ عَلَى الرَّقْمِ 5550142.",
        "",
        "شُكْرًا لِتَعَاوُنِكُمْ،",
        "إِدَارَةُ العِمَارَةِ",
    ]
    f, fb = font(AR_AMIRI, 40), font(AR_AMIRI_BOLD, 52)
    img, d = page(1240, 860)
    y = 90
    for i, s in enumerate(lines):
        if s:
            text_ar(d, (1150, y), s, fb if i == 0 else f)
        y += 58
    save(img, "ar1_clean_diacritics")
    write_txt("ar1_clean_diacritics", [s for s in lines if s])


def ar_small():
    lines = [
        "إعلان عن اجتماع الجمعية السنوية",
        "يسر مجلس الإدارة دعوة جميع الأعضاء إلى الاجتماع السنوي للجمعية",
        "وذلك في تمام الساعة 6:30 مساء يوم الخميس 23 تشرين الثاني 2025",
        "في قاعة المؤتمرات بالطابق الثالث من مبنى الجمعية.",
        "جدول الأعمال:",
        "1 قراءة محضر الاجتماع السابق والمصادقة عليه.",
        "2 عرض التقرير المالي للسنة المنتهية في 31 كانون الأول 2024.",
        "3 مناقشة ميزانية العام القادم ورسم الاشتراك.",
        "4 انتخاب عضوين جديدين لمجلس الإدارة.",
        "5 ما يستجد من أعمال.",
        "يرجى إحضار بطاقة العضوية، وفي حال تعذر الحضور يمكن توكيل عضو آخر خطيا",
        "بإرسال رسالة إلى أمين السر قبل الاجتماع بثلاثة أيام على الأقل.",
        "ستقدم المرطبات الخفيفة بعد الاجتماع، والمواقف مجانية للأعضاء.",
        "تفتح الأبواب في تمام الساعة 6:00 مساء ويبدأ الاجتماع في موعده.",
        "سيتم نشر المحضر على لوحة الإعلانات خلال 10 أيام.",
    ]
    f, fb = font(AR_NASKH, 19), font(AR_NASKH_BOLD, 21)
    img, d = page(760, 520)
    y = 30
    for i, s in enumerate(lines):
        text_ar(d, (730, y), s, fb if i == 0 else f)
        y += 31
    save(img, "ar2_small_font")
    write_txt("ar2_small_font", lines)


def ar_columns():
    right = [
        "مواعيد المكتبة",
        "تفتح المكتبة أبوابها من السبت إلى",
        "الخميس، من الساعة 9 صباحا حتى",
        "الساعة 8 مساء، وتغلق يوم الجمعة.",
        "يمكن استعارة 5 كتب لمدة 14 يوما،",
        "ويمكن تمديد المدة مرة واحدة.",
    ]
    left = [
        "قواعد الاستعارة",
        "يجب إحضار بطاقة المكتبة عند كل",
        "استعارة. يرجى الحفاظ على الهدوء",
        "داخل قاعات القراءة، ويمنع إدخال",
        "الطعام والمشروبات إلى القاعات.",
        "لأي استفسار اتصل على 5550188.",
    ]
    f, fb = font(AR_AMIRI, 32), font(AR_AMIRI_BOLD, 38)
    img, d = page(1100, 480)
    for xr, col in ((1030, right), (540, left)):
        y = 60
        for i, s in enumerate(col):
            text_ar(d, (xr, y), s, fb if i == 0 else f)
            y += 58
    d.line([(550, 50), (550, 440)], fill=(160, 160, 160), width=2)
    save(img, "ar3_two_columns")
    # Reading order: right column first, then left.
    write_txt("ar3_two_columns", right + left)


def ar_table():
    title = "فاتورة رقم 20457"
    date = "التاريخ: 5 تشرين الثاني 2025"
    head = ["الصنف", "الكمية", "السعر", "المجموع"]
    rows = [
        ["دفتر ملاحظات", "4", "12", "48"],
        ["قلم حبر", "10", "3", "30"],
        ["ملف بلاستيكي", "6", "5", "30"],
        ["حقيبة مكتب", "1", "85", "85"],
        ["ورق طباعة", "5", "22", "110"],
    ]
    total = ["الإجمالي", "", "", "303"]
    f, fb = font(AR_NASKH, 28), font(AR_NASKH_BOLD, 28)
    img, d = page(1000, 640)
    text_ar(d, (940, 36), title, font(AR_NASKH_BOLD, 36))
    text_ar(d, (940, 100), date, f)
    # Right-aligned columns, first cell at the right edge (RTL).
    xr = [940, 470, 350, 200]
    y = 168
    d.line([(50, y - 8), (950, y - 8)], fill=INK, width=2)
    for r, row in enumerate([head] + rows + [total]):
        for c, cell in enumerate(row):
            if cell:
                fn = fb if r in (0, len(rows) + 1) else f
                text_ar(d, (xr[c], y), cell, fn)
        y += 56
        d.line([(50, y - 8), (950, y - 8)], fill=(150, 150, 150), width=1)
    save(img, "ar4_table_list")
    out = [title, date, " ".join(head)]
    out += [" ".join(r) for r in rows]
    out += ["الإجمالي 303"]
    write_txt("ar4_table_list", out)


def ar_noisy():
    lines = [
        "إيصال استلام",
        "رقم الإيصال: 88213",
        "التاريخ: 12 تشرين الثاني 2025",
        "استلمنا مبلغ 250 مقابل خدمات الصيانة",
        "طريقة الدفع: نقدا",
        "المستلم: قسم المحاسبة",
        "يحتفظ المستلم بنسخة من هذا الإيصال",
        "شكرا لتعاملكم معنا",
    ]
    f, fb = font(AR_NASKH, 38), font(AR_NASKH_BOLD, 50)
    img, d = page(900, 600, bg=(238, 234, 224))
    y = 50
    for i, s in enumerate(lines):
        text_ar(d, (840, y), s, fb if i == 0 else f)
        y += 66
    save(img, "ar5_noisy_skewed")
    write_txt("ar5_noisy_skewed", lines)
    photo_effect("ar5_noisy_skewed", rotate="3", seed="23")


if __name__ == "__main__":
    for fn in (en_clean, en_small, en_columns, en_table, en_noisy,
               ar_clean_diacritics, ar_small, ar_columns, ar_table, ar_noisy):
        fn()
    print("done:", len(list(OUT.glob("*.png"))), "images")
