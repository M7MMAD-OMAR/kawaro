"""Writes fixtures/cases/text_cases.jsonl: 48 text cases, 8 categories x (3 ar + 3 en).
Stdlib only. Run: python3 -I fixtures/make_text_cases.py"""
import json, os

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "cases", "text_cases.jsonl")
SYS = {
    "extract": "You extract fields from document text. Reply with one JSON object only, using exactly the requested keys. Numbers are plain numbers, dates are YYYY-MM-DD.",
    "classify": "You classify a document by its text. Reply with exactly one label from: invoice, receipt, contract, resume, letter, report. No other words.",
    "rename": "You propose a file name from the document content. Reply with the file name only: lowercase English words joined by underscores, no spaces, keep the given extension.",
    "summarize": "Summarize the text in exactly one sentence. Reply with the sentence only.",
    "translate": "Translate the sentence. Reply with the translation only.",
    "rewrite": "Rewrite the title as asked. Reply with the new title only.",
    "tool_call": 'Pick one tool and reply with one JSON object only: {"tool": name, "args": {...}}. Tools: convert(file, format), compress(file, quality), merge_pdf(files), extract_audio(file, format). If no tool fits, reply {"tool": "none", "args": {}}.',
    "refuse_or_unknown": "Answer only from the text given. If the answer needs outside knowledge or information not present, say you cannot know it. Do not guess.",
}
C = []
def add(cat, lang, user, gold, check, mt=96):
    C.append({"category": cat, "lang": lang, "system": SYS[cat], "user": user, "gold": gold, "check": check, "max_tokens": mt})

# extract
add("extract","ar","استخرج الحقول invoice_no و vendor و date و total من النص:\nفاتورة رقم 4471\nالمورد: شركة النور للتجهيزات\nالتاريخ: 2025-03-14\nالإجمالي: 1250.50 ريال",
    {"fields":{"invoice_no":"4471","vendor":"شركة النور للتجهيزات","date":"2025-03-14","total":1250.5}},"json_fields")
add("extract","ar","استخرج الحقول store و date و total و items_count من الإيصال:\nمتجر الأمل\n2025-07-02\nخبز 2 ، حليب 1 ، جبن 1\nالمجموع 38.75",
    {"fields":{"store":"متجر الأمل","date":"2025-07-02","total":38.75,"items_count":3}},"json_fields")
add("extract","ar","استخرج الحقول name و phone و city من النص:\nالاسم: ليلى حداد\nالهاتف: 0501234567\nالمدينة: دبي",
    {"fields":{"name":"ليلى حداد","phone":"0501234567","city":"دبي"}},"json_fields")
add("extract","en","Extract the keys invoice_no, vendor, date, total from:\nInvoice #8812\nVendor: Brightline Supplies Ltd\nDate: 2025-11-30\nTotal due: 940.00 USD",
    {"fields":{"invoice_no":"8812","vendor":"Brightline Supplies Ltd","date":"2025-11-30","total":940}},"json_fields")
add("extract","en","Extract the keys store, date, total, items_count from this receipt:\nGreen Basket Market\n2025-01-09\nApples x3, Rice x1, Tea x2\nTOTAL 21.40",
    {"fields":{"store":"Green Basket Market","date":"2025-01-09","total":21.4,"items_count":3}},"json_fields")
add("extract","en","Extract the keys name, email, role from:\nName: Omar Rahman\nContact: omar.rahman@example.org\nRole: Site engineer",
    {"fields":{"name":"Omar Rahman","email":"omar.rahman@example.org","role":"Site engineer"}},"json_fields")

# classify
add("classify","ar","عقد إيجار بين المؤجر والمستأجر لمدة سنة واحدة، يلتزم المستأجر بدفع الأجرة شهريا ويحق للمؤجر إنهاء العقد عند التأخر.",{"label":"contract","labels":["invoice","receipt","contract","resume","letter","report"]},"keywords",16)
add("classify","ar","السيرة الذاتية: خالد منصور، خبرة سبع سنوات في المحاسبة، بكالوريوس تجارة، يجيد الإنجليزية والإكسل.",{"label":"resume","labels":["invoice","receipt","contract","resume","letter","report"]},"keywords",16)
add("classify","ar","تقرير الأداء الربع سنوي: ارتفعت المبيعات بنسبة 12 بالمئة وانخفضت التكاليف التشغيلية، والتوصيات في القسم الأخير.",{"label":"report","labels":["invoice","receipt","contract","resume","letter","report"]},"keywords",16)
add("classify","en","Invoice 2291. Bill to: Harbor Print Co. Item: 500 flyers. Amount due within 30 days: 310.00 EUR.",{"label":"invoice","labels":["invoice","receipt","contract","resume","letter","report"]},"keywords",16)
add("classify","en","Dear Ms. Alvarez, thank you for your time last week. I am writing to confirm the meeting on Thursday. Kind regards, Sam.",{"label":"letter","labels":["invoice","receipt","contract","resume","letter","report"]},"keywords",16)
add("classify","en","Thank you for shopping at Corner Cafe. Latte 4.50, Muffin 3.00. Paid by card. Total 7.50.",{"label":"receipt","labels":["invoice","receipt","contract","resume","letter","report"]},"keywords",16)

# rename
add("rename","ar","الامتداد: pdf\nالمحتوى: فاتورة رقم 4471 من شركة النور للتجهيزات بتاريخ مارس 2025.\nاسم الملف الحالي: scan001.pdf",{"patterns":[r"^[a-z0-9][a-z0-9_.\-]*\.pdf$"],"tokens_any":["invoice","4471"],"max_len":60},"regex",32)
add("rename","ar","الامتداد: docx\nالمحتوى: عقد إيجار شقة لمدة سنة بين الطرفين، يبدأ في يناير 2026.\nاسم الملف الحالي: New Document (3).docx",{"patterns":[r"^[a-z0-9][a-z0-9_.\-]*\.docx$"],"tokens_any":["contract","lease","rental"],"max_len":60},"regex",32)
add("rename","ar","الامتداد: jpg\nالمحتوى: صورة مخطط مبيعات شهري بأعمدة لعام 2025.\nاسم الملف الحالي: IMG_5521.jpg",{"patterns":[r"^[a-z0-9][a-z0-9_.\-]*\.jpg$"],"tokens_any":["sales","chart"],"max_len":60},"regex",32)
add("rename","en","Extension: pdf\nContent: Quarterly budget report for the marketing team, Q3 2025.\nCurrent name: Document final FINAL (2).pdf",{"patterns":[r"^[a-z0-9][a-z0-9_.\-]*\.pdf$"],"tokens_any":["budget","report"],"max_len":60},"regex",32)
add("rename","en","Extension: png\nContent: Screenshot of the login page error message 'invalid password'.\nCurrent name: Screenshot from 2025-05-04 10-22-11.png",{"patterns":[r"^[a-z0-9][a-z0-9_.\-]*\.png$"],"tokens_any":["login","error"],"max_len":60},"regex",32)
add("rename","en","Extension: txt\nContent: Meeting notes: kickoff with the design agency, action items for April.\nCurrent name: untitled.txt",{"patterns":[r"^[a-z0-9][a-z0-9_.\-]*\.txt$"],"tokens_any":["meeting","notes","kickoff"],"max_len":60},"regex",32)

# summarize
add("summarize","ar","افتتحت البلدية مكتبة جديدة في الحي القديم. تضم المكتبة عشرة آلاف كتاب وقاعة للأطفال. الدخول مجاني لجميع السكان.",{"any_groups":[["مكتبة","المكتبة"],["مجان","مجاني"]],"max_sentences":1,"max_words":30},"keywords",96)
add("summarize","ar","تأخرت الرحلة ثلاث ساعات بسبب العاصفة. أبلغت الشركة الركاب عبر الرسائل. عُرض على المتضررين حجز فندقي مجاني.",{"any_groups":[["الرحلة","رحلة"],["العاصفة","عاصف","طقس"]],"max_sentences":1,"max_words":30},"keywords",96)
add("summarize","ar","أطلقت الشركة تطبيقا جديدا لتحويل الملفات. يعمل التطبيق دون اتصال بالإنترنت. لا تُرسل الملفات إلى أي خادم.",{"any_groups":[["تطبيق"],["دون اتصال","بدون اتصال","محلي","لا تُرسل","لا ترسل","خادم"]],"max_sentences":1,"max_words":30},"keywords",96)
add("summarize","en","The city council approved a new bike lane on Main Street. Construction starts in June and lasts four months. Parking will be reduced by twenty spaces.",{"any_groups":[["bike lane","bike"],["main street","main st"]],"max_sentences":1,"max_words":35},"keywords",96)
add("summarize","en","Our server crashed at 2 AM because the disk filled up. The team cleared old logs and restarted it by 3 AM. A disk alert has been added to prevent a repeat.",{"any_groups":[["disk"],["crash","outage","down","failed"]],"max_sentences":1,"max_words":35},"keywords",96)
add("summarize","en","Maya joined the company as a designer in 2021. She led the redesign of the mobile app. This year she was promoted to design lead.",{"any_groups":[["maya"],["promot","design lead","lead"]],"max_sentences":1,"max_words":35},"keywords",96)

# translate
add("translate","ar","Translate to English: الملف محفوظ في المجلد الأزرق.",{"any_groups":[["file"],["saved","stored","kept"],["blue"],["folder"]]},"keywords",48)
add("translate","ar","Translate to English: أين أجد فاتورة شهر آذار؟",{"any_groups":[["where"],["invoice","bill"],["march"]]},"keywords",48)
add("translate","ar","Translate to English: اكتمل التحويل ويمكنك فتح الملف الجديد.",{"any_groups":[["conver"],["complet","finish","done"],["open"],["file"]]},"keywords",48)
add("translate","en","Translate to Arabic: The file was saved next to the original.",{"any_groups":[["الملف"],["حفظ","محفوظ","حُفظ","تم حفظ"],["الأصل","الأصلي","الاصلي","الاصل"]]},"keywords",64)
add("translate","en","Translate to Arabic: Please choose the output format.",{"any_groups":[["اختر","يرجى","الرجاء","رجاء"],["صيغة","تنسيق","امتداد","الإخراج","الناتج","المخرج"]]},"keywords",64)
add("translate","en","Translate to Arabic: Your invoice is ready.",{"any_groups":[["فاتورة","فاتورتك"],["جاهز","جاهزة","اكتمل"]]},"keywords",48)

# rewrite
add("rewrite","ar","اختصر العنوان إلى خمس كلمات أو أقل مع الحفاظ على المعنى: تقرير شامل ومفصل عن نتائج مبيعات الشركة خلال الربع الثالث من العام",{"max_words":5,"any_groups":[["مبيعات"]]},"keywords",32)
add("rewrite","ar","صحح الخطأ الإملائي في العنوان وأعده فقط: تقرير المبيعات الشهري لشهر مارس (المبيعاة)",{"any_groups":[["المبيعات"],["تقرير"]],"forbid":["المبيعاة"]},"keywords",32)
add("rewrite","ar","اختصر العنوان إلى ست كلمات أو أقل: دليل المستخدم الكامل لاستخدام تطبيق تحويل الملفات على الحاسوب",{"max_words":6,"any_groups":[["دليل"],["تطبيق","الملفات","تحويل"]]},"keywords",32)
add("rewrite","en","Shorten this title to at most 5 words, keep the meaning: A Complete and Detailed Guide to Converting Your Files Offline",{"max_words":5,"any_groups":[["convert","offline"],["guide"]]},"keywords",32)
add("rewrite","en","Fix the spelling mistakes in this title and return only the title: Quarterly sales reprot for Janury",{"any_groups":[["report"],["january"]],"forbid":["reprot","janury"]},"keywords",32)
add("rewrite","en","Shorten this title to at most 4 words: Meeting Notes From the Monday Morning Planning Session",{"max_words":4,"any_groups":[["meeting","notes","planning"]]},"keywords",32)

# tool_call
add("tool_call","ar","حوّل الملف photo.heic إلى png",{"exact":{"tool":"convert","args":{"file":"photo.heic","format":"png"}}},"json_fields",64)
add("tool_call","ar","اضغط الملف scan.pdf بجودة 60",{"exact":{"tool":"compress","args":{"file":"scan.pdf","quality":60}}},"json_fields",64)
add("tool_call","ar","ما حال الطقس اليوم في الرياض؟",{"exact":{"tool":"none","args":{}}},"json_fields",64)
add("tool_call","en","Merge a.pdf and b.pdf into one document",{"exact":{"tool":"merge_pdf","args":{"files":["a.pdf","b.pdf"]}}},"json_fields",64)
add("tool_call","en","Extract the audio from talk.mp4 as mp3",{"exact":{"tool":"extract_audio","args":{"file":"talk.mp4","format":"mp3"}}},"json_fields",64)
add("tool_call","en","Send this file to my manager by email",{"exact":{"tool":"none","args":{}}},"json_fields",64)

# refuse_or_unknown
add("refuse_or_unknown","ar","النص: تم تحويل الملف بنجاح.\nالسؤال: كم كان حجم الملف الأصلي بالميغابايت؟",{"expect":"cannot_know"},"flag",64)
add("refuse_or_unknown","ar","ما هو سعر سهم أرامكو غدا عند الإغلاق؟",{"expect":"cannot_know"},"flag",64)
add("refuse_or_unknown","ar","النص: اجتمع الفريق يوم الأحد.\nالسؤال: ما اسم مدير الفريق؟",{"expect":"cannot_know"},"flag",64)
add("refuse_or_unknown","en","Text: The invoice was paid in full.\nQuestion: What was the credit card number used?",{"expect":"cannot_know"},"flag",64)
add("refuse_or_unknown","en","What will the weather be in Berlin at noon next Friday?",{"expect":"cannot_know"},"flag",64)
add("refuse_or_unknown","en","Text: The report covers Q2 results.\nQuestion: Who is the CEO of the company that wrote it?",{"expect":"cannot_know"},"flag",64)

assert len(C) == 48
n = {}
for c in C:
    k = (c["category"], c["lang"]); n[k] = n.get(k, 0) + 1
    c["id"] = "t%02d_%s_%s%d" % (len(n) and sum(n.values()), c["category"], c["lang"], n[k])
assert all(v == 3 for v in n.values()) and len(n) == 16
with open(OUT, "w", encoding="utf-8") as f:
    for c in C:
        f.write(json.dumps({k: c[k] for k in ["id","category","lang","system","user","gold","check","max_tokens"]}, ensure_ascii=False) + "\n")
print("wrote", len(C))
