"""Checker for fixtures/cases/vl_cases.jsonl. Stdlib only, python3 -I compatible.
score(case, output_text) -> (ok, reason). Checks: json_fields, table, keywords."""
import json, re, unicodedata

def norm(s):
    s = unicodedata.normalize("NFKC", str(s))
    s = re.sub(r"[ً-ٰٟـ]", "", s)
    s = re.sub("[أإآ]", "ا", s)
    s = s.replace("ى", "ي").replace("ة", "ه")
    return re.sub(r"\s+", " ", s).strip().lower()

def strip_wrap(t):
    t = re.sub(r"<think>.*?</think>", "", t or "", flags=re.S).strip()
    return re.sub(r"^```[a-zA-Z]*\s*|\s*```$", "", t).strip()

def parse_json(text):
    t = strip_wrap(text)
    for cand in (t, t[t.find("{"):t.rfind("}") + 1] if "{" in t else ""):
        try:
            return json.loads(cand)
        except Exception:
            pass
    return None

def eq(exp, act):
    if isinstance(exp, (int, float)) and not isinstance(exp, bool):
        try:
            return abs(float(str(act).replace(",", "")) - float(exp)) < 0.005
        except Exception:
            return False
    e, a = norm(exp), norm(act)
    return e == a or e in a

def c_json(g, out):
    obj = parse_json(out)
    if not isinstance(obj, dict):
        return False, "no JSON object"
    for k, v in g["fields"].items():
        if k not in obj:
            return False, "missing " + k
        if not eq(v, obj[k]):
            return False, "wrong %s: %r" % (k, obj[k])
    return True, "ok"

def parse_table(text):
    rows = []
    for ln in strip_wrap(text).splitlines():
        if "|" not in ln:
            continue
        cells = [c.strip() for c in ln.strip().strip("|").split("|")]
        if all(re.fullmatch(r":?-{2,}:?", c) for c in cells if c):
            continue
        rows.append(cells)
    return rows

def c_table(g, out):
    rows = parse_table(out)
    if len(rows) != len(g["rows"]):
        return False, "%d rows, expected %d" % (len(rows), len(g["rows"]))
    for i, (r, e) in enumerate(zip(rows, g["rows"])):
        if len(r) != len(e):
            return False, "row %d has %d cells" % (i, len(r))
        for j, (a, b) in enumerate(zip(r, e)):
            if norm(a) != norm(b) and not (re.fullmatch(r"[\d.]+", b) and eq(float(b), a)):
                return False, "cell %d,%d: %r != %r" % (i, j, a, b)
    return True, "ok"

def c_keywords(g, out):
    n = norm(strip_wrap(out))
    for grp in g["all"]:
        hit = False
        for w in grp:
            if w.startswith("#"):
                hit = re.search(r"(?<![\d.])" + re.escape(w[1:]) + r"(?![\d])", n) is not None
            else:
                hit = norm(w) in n
            if hit:
                break
        if not hit:
            return False, "none of %s" % grp
    return True, "ok"

def score(case, output_text):
    fn = {"json_fields": c_json, "table": c_table, "keywords": c_keywords}[case["check"]]
    return fn(case["gold"], output_text or "")
