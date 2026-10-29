"""Checker for fixtures/cases/text_cases.jsonl. Stdlib only, python3 -I compatible.
score(case, output_text) -> (ok, reason). Strict, deterministic, no model calls."""
import json, re, unicodedata

REFUSE_EN = ["cannot", "can't", "can not", "don't know", "do not know", "unable", "not able",
             "not provided", "not mentioned", "not specified", "no information", "doesn't say",
             "does not say", "does not mention", "not stated", "impossible to know", "cannot be determined",
             "don't have", "do not have", "no way to"]
REFUSE_AR = ["لا أستطيع", "لا استطيع", "لا يمكنني", "لا يمكن", "لا أعرف", "لا اعرف", "لا أعلم", "لا اعلم",
             "لا أملك", "لا املك", "ليس لدي", "غير قادر", "غير متوفر", "لا تتوفر", "لم يذكر", "لم يرد",
             "غير مذكور", "غير محدد", "لا توجد معلومات", "لا يوجد في النص", "لا تحتوي"]

def norm(s):
    s = unicodedata.normalize("NFKC", str(s))
    s = re.sub(r"[ً-ٰٟـ]", "", s)          # tashkeel, tatweel
    s = re.sub("[أإآ]", "ا", s)             # alef forms
    s = s.replace("ى", "ي").replace("ة", "ه")
    return re.sub(r"\s+", " ", s).strip().lower()

def strip_wrap(t):
    t = t.strip()
    t = re.sub(r"^```[a-zA-Z]*\s*|\s*```$", "", t).strip()
    t = re.sub(r"<think>.*?</think>", "", t, flags=re.S).strip()
    return t.strip("`\"' \n")

def parse_json(text):
    t = strip_wrap(text)
    try:
        return json.loads(t)
    except Exception:
        pass
    a, b = t.find("{"), t.rfind("}")
    if a >= 0 and b > a:
        try:
            return json.loads(t[a:b + 1])
        except Exception:
            return None
    return None

def eq(exp, act):
    if isinstance(exp, bool) or isinstance(act, bool):
        return exp == act
    if isinstance(exp, (int, float)):
        try:
            return abs(float(str(act).replace(",", "")) - float(exp)) < 0.005
        except Exception:
            return False
    if isinstance(exp, str):
        e, a = norm(exp), norm(act)
        return e == a or e in a
    if isinstance(exp, list):
        return isinstance(act, list) and len(exp) == len(act) and all(eq(x, y) for x, y in zip(exp, act))
    if isinstance(exp, dict):
        return isinstance(act, dict) and set(exp) == set(act) and all(eq(exp[k], act[k]) for k in exp)
    return exp == act

def words(t):
    return len(re.findall(r"\S+", t))

def sentences(t):
    return len([x for x in re.split(r"[.!?؟۔]+(?:\s|$)", t.strip()) if x.strip()])

def c_json(g, out):
    obj = parse_json(out)
    if not isinstance(obj, dict):
        return False, "no JSON object"
    if "exact" in g:
        return (True, "ok") if eq(g["exact"], obj) else (False, "json differs: %s" % json.dumps(obj, ensure_ascii=False)[:120])
    for k, v in g["fields"].items():
        if k not in obj:
            return False, "missing key " + k
        if not eq(v, obj[k]):
            return False, "wrong %s: %r" % (k, obj[k])
    extra = set(obj) - set(g["fields"])
    if extra:
        return False, "extra keys %s" % sorted(extra)
    return True, "ok"

def c_keywords(g, out):
    t = strip_wrap(out)
    n = norm(t)
    if "label" in g:
        found = [l for l in g["labels"] if re.search(r"\b" + l + r"\b", n)]
        return (True, "ok") if found == [g["label"]] else (False, "labels found %s" % found)
    for grp in g.get("any_groups", []):
        if not any(norm(w) in n for w in grp):
            return False, "none of %s" % grp
    for w in g.get("forbid", []):
        if norm(w) in n:
            return False, "forbidden " + w
    if "max_words" in g and words(t) > g["max_words"]:
        return False, "%d words > %d" % (words(t), g["max_words"])
    if "max_sentences" in g and sentences(t) > g["max_sentences"]:
        return False, "%d sentences" % sentences(t)
    if not t:
        return False, "empty"
    return True, "ok"

def c_regex(g, out):
    t = strip_wrap(out).splitlines()
    t = t[0].strip().strip("`\"'") if t else ""
    if not t:
        return False, "empty"
    for p in g["patterns"]:
        if not re.search(p, t):
            return False, "pattern %s fails on %r" % (p, t)
    if "max_len" in g and len(t) > g["max_len"]:
        return False, "too long"
    if g.get("tokens_any") and not any(k in t for k in g["tokens_any"]):
        return False, "no token of %s in %r" % (g["tokens_any"], t)
    return True, "ok"

def c_flag(g, out):
    n = norm(out)
    hit = any(norm(m) in n for m in REFUSE_EN + REFUSE_AR)
    return (True, "ok") if hit else (False, "no cannot-know marker")

def score(case, output_text):
    fn = {"json_fields": c_json, "keywords": c_keywords, "regex": c_regex, "flag": c_flag}[case["check"]]
    return fn(case["gold"], output_text or "")

if __name__ == "__main__":
    import sys
    # usage: check_text.py cases.jsonl outputs.jsonl  (outputs: {"id","output"} per line)
    cases = {json.loads(l)["id"]: json.loads(l) for l in open(sys.argv[1], encoding="utf-8")}
    ok = tot = 0
    for l in open(sys.argv[2], encoding="utf-8"):
        r = json.loads(l); res = score(cases[r["id"]], r["output"]); tot += 1; ok += res[0]
        print(r["id"], "PASS" if res[0] else "FAIL", res[1])
    print("%d/%d" % (ok, tot))
