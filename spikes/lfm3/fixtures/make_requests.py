"""Write requests.jsonl and check every line against actions.json.

Usage: python3 -I spikes/lfm3/fixtures/make_requests.py
Asserts: the action id exists, file extensions fit `accepts`, `skip_ext` and
`min_files` hold, and every expected param is inside its range or enum.
"""
import json, sys, pathlib, collections
root = pathlib.Path(__file__).resolve().parents[3]
cat = {a["id"]: a for a in json.load(open(root/"spikes/lfm3/fixtures/actions.json"))}
EXT = {
 "image": "png jpg jpeg webp avif gif bmp tif tiff ico".split(),
 "audio": "mp3 ogg opus flac wav m4a aac".split(),
 "video": "mp4 mkv webm mov avi".split(),
 "document": "doc docx odt rtf txt xls xlsx ods ppt pptx odp csv".split(),
 "pdf": ["pdf"],
}
ar = [
 ("حوّل الصورة إلى WebP", ["photo.png"], "image.to-webp", {}),
 ("اضغط ملف PDF لأقل حجم ممكن حتى أرسله بالبريد", ["report.pdf"], "pdf.compress", {"preset": "screen"}),
 ("ادمج هذه الملفات في ملف PDF واحد", ["a.pdf", "b.pdf", "c.pdf"], "pdf.merge", {}),
 ("حوّل الأغنية إلى MP3", ["song.wav"], "audio.to-mp3", {}),
 ("استخرج الصوت من هذا الفيديو بصيغة MP3", ["clip.mp4"], "video.extract-mp3", {}),
 ("حوّل ملف الوورد إلى PDF", ["letter.docx"], "documents.to-pdf", {}),
 ("صغّر الصورة إلى 25 بالمئة من حجمها", ["holiday.jpg"], "image.resize-50", {"scale": 25}),
 ("بدي صغّر حجم هالفيديو، بس لا تخليه أعلى من 720", ["wedding.mov"], "video.compress", {"max_height": 720}),
 ("لفّلي هالصورة 180 درجة", ["scan.jpg"], "image.rotate-90", {"degrees": 180}),
 ("ابغى أحول هالـ PDF إلى صور بدقة 300", ["menu.pdf"], "pdf.to-images", {"dpi": 300}),
 ("اقلب الصورة عمودياً", ["logo.png"], "image.flip-h", {"axis": "vertical"}),
 ("حوّل الفيديو إلى صورة GIF متحركة بمعدل 10 إطارات في الثانية", ["demo.mp4"], "video.to-gif", {"fps": 10}),
 ("قسّم ملف الـ PDF إلى ملف لكل صفحة", ["book.pdf"], "pdf.split", {}),
 ("حوّل جدول الإكسل إلى صيغة ODS", ["budget.xlsx"], "documents.to-ods", {}),
 ("ترجم هذا المستند إلى الإنجليزية", ["contract.docx"], "none", {}),
]
en = [
 ("Convert this photo to JPG with quality 60", ["photo.png"], "image.to-jpg", {"quality": 60}),
 ("Combine these pictures into one PDF", ["a.png", "b.png", "c.png"], "image.to-pdf", {}),
 ("Compress this PDF but keep it good enough to print", ["invoice.pdf"], "pdf.compress", {"preset": "printer"}),
 ("Turn every page of this PDF upside down", ["scan.pdf"], "pdf.rotate", {"degrees": 180}),
 ("Make a FLAC copy of this recording", ["interview.wav"], "audio.to-flac", {}),
 ("I need an Opus version of this for voice notes", ["voice.m4a"], "audio.to-opus", {}),
 ("Turn the video into a WebM for the web", ["trailer.mp4"], "video.to-webm", {}),
 ("Rip the soundtrack from this video as WAV", ["concert.mkv"], "video.extract-wav", {}),
 ("Convert these two files to ODT", ["notes.docx", "memo.rtf"], "documents.to-odt", {}),
 ("Make the image at most 1280 pixels on the longest side", ["banner.png"], "image.fit-1920", {"max_side": 1280}),
 ("Strip the EXIF and location data from my photos", ["IMG_0412.jpg", "IMG_0413.jpg"], "image.strip", {}),
 ("Convert the clip to a GIF 320 pixels wide", ["reaction.mp4"], "video.to-gif", {"width": 320}),
 ("Make this PNG black and white", ["portrait.png"], "image.grayscale", {}),
 ("Make it look better", ["photo.png"], "none", {}),
 ("Email this to my boss", ["report.pdf"], "none", {}),
]
assert len(ar) == len(en) == 15
rows = []
for i in range(15):
    rows.append(("ar", ar[i])); rows.append(("en", en[i]))
out = []
for n, (lang, (text, files, act, params)) in enumerate(rows, 1):
    if act != "none":
        a = cat[act]
        for f in files:
            e = f.rsplit(".", 1)[1]
            assert any(e in EXT[k] for k in a["accepts"]), (act, f)
            assert e not in a.get("skip_ext", []), (act, f)
        assert len(files) >= a.get("min_files", 1), act
        for k, v in params.items():
            p = a["params"][k]
            if "enum" in p: assert v in p["enum"], (act, k, v)
            else: assert p.get("min", v) <= v <= p.get("max", v), (act, k, v)
    out.append(json.dumps({"id": n, "lang": lang, "text": text, "files": files,
                           "expected_action": act, "expected_params": params}, ensure_ascii=False))
p = root/"spikes/lfm3/fixtures/requests.jsonl"
p.write_text("\n".join(out) + "\n", encoding="utf-8")
c = collections.Counter(json.loads(l)["expected_action"] for l in out)
print(len(out)); print(dict(c))
