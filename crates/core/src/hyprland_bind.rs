//! The Hyprland drop key bind as generated text (S-111).
//!
//! Kawaro writes the bind into its own file (`kawaro.lua` or `kawaro.conf`)
//! between two marker lines and never edits the user's own Hyprland config.
//! Everything here is pure: the CLI does the file and `hyprctl` work.

use crate::hold::BindState;
use crate::i18n::Lang;

/// Which Hyprland config language the generated file is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syntax {
    Lua,
    Hyprlang,
}

const FALLBACK_BINARY: &str = "/usr/bin/kawaro";

/// The binary as written into the bind: a path with `.local` or `$HOME` in
/// it becomes the bare command name, an empty one the packaged path.
pub fn normalise_binary(path: &str) -> String {
    if path.trim().is_empty() {
        return FALLBACK_BINARY.to_string();
    }
    if path.contains(".local") || path.contains("$HOME") {
        return path
            .rsplit(['/', '\\'])
            .next()
            .filter(|n| !n.is_empty())
            .unwrap_or("kawaro")
            .to_string();
    }
    path.to_string()
}

/// A drop key as the bind syntax names it: modifiers, then the key.
struct Chord {
    mods: Vec<String>,
    key: String,
}

fn parse_key(key: &str) -> Chord {
    if key.eq_ignore_ascii_case("alt") {
        return Chord {
            mods: vec![],
            key: "ALT_L".into(),
        };
    }
    let mut parts: Vec<String> = key.split('+').map(|p| p.trim().to_uppercase()).collect();
    let key = parts.pop().unwrap_or_default();
    Chord { mods: parts, key }
}

/// The generated block for `key` (`Alt`, `Super+Z`, ...), markers included,
/// ending with a newline.
pub fn hyprland_bind_text(key: &str, syntax: Syntax, kawaro_path: &str) -> String {
    let bin = normalise_binary(kawaro_path);
    let chord = parse_key(key);
    let alt = chord.mods.is_empty() && chord.key == "ALT_L";
    match syntax {
        Syntax::Lua => {
            let keys = chord
                .mods
                .iter()
                .chain(std::iter::once(&chord.key))
                .cloned()
                .collect::<Vec<_>>()
                .join(" + ");
            let (press_opts, release_label, press_label) = if alt {
                (
                    "non_consuming = true, ignore_mods = true",
                    "Kawaro: release Alt to close",
                    "Kawaro: hold Alt to drop",
                )
            } else {
                (
                    "non_consuming = true",
                    "Kawaro: release to close",
                    "Kawaro: hold to drop",
                )
            };
            let mut t = format!(
                "-- kawaro:begin\n\
                 hl.bind(\"{keys}\", hl.dsp.exec_cmd(\"{bin} drop --hold --at \\\"$(hyprctl cursorpos)\\\"\"), {{ {press_opts}, description = \"{press_label}\" }})\n\
                 hl.bind(\"{keys}\", hl.dsp.exec_cmd(\"{bin} drop --release\"), {{ release = true, non_consuming = true, ignore_mods = true, description = \"{release_label}\" }})\n"
            );
            if alt {
                t.push_str(&format!(
                    "local alt_down = false\n\
                     hl.on(\"input.keyboard.key\", function(code, _, state)\n  \
                     if code == 64 then -- Alt_L, the evdev code 56 plus 8\n    \
                     alt_down = state == 1\n  \
                     elseif alt_down and state == 1 then\n    \
                     hl.exec_cmd(\"sleep 0.05; {bin} drop --other-key\")\n  \
                     end\n\
                     end)\n"
                ));
            }
            t.push_str("-- kawaro:end\n");
            t
        }
        Syntax::Hyprlang => {
            let mods = chord.mods.join(" ");
            let press = if alt { "bindni" } else { "bindn" };
            let (m, k) = (mods.as_str(), chord.key.as_str());
            format!(
                "# kawaro:begin\n\
                 {press} = {m}, {k}, exec, {bin} drop --hold --at \"$(hyprctl cursorpos)\"\n\
                 bindrni = {m}, {k}, exec, {bin} drop --release\n\
                 # kawaro:end\n"
            )
        }
    }
}

/// Why a block could not be merged into a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeError {
    /// A begin marker without its end marker, or the reverse.
    Unbalanced,
    /// More than one block.
    Duplicate,
}

fn marker(line: &str, which: &str) -> bool {
    let t = line.trim();
    t == format!("-- kawaro:{which}") || t == format!("# kawaro:{which}")
}

/// Puts `block` into `existing`: appended when there is no block, replacing
/// the old one when there is. Text outside the markers stays byte for byte.
pub fn merge_block(existing: &str, block: &str) -> Result<String, MergeError> {
    let (mut begins, mut ends) = (vec![], vec![]);
    let mut pos = 0;
    for line in existing.split_inclusive('\n') {
        let body = line.trim_end_matches(['\n', '\r']);
        if marker(body, "begin") {
            begins.push((pos, pos + line.len()));
        } else if marker(body, "end") {
            ends.push((pos, pos + line.len()));
        }
        pos += line.len();
    }
    match (begins.as_slice(), ends.as_slice()) {
        ([], []) => {
            let mut out = existing.to_string();
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(block);
            Ok(out)
        }
        ([b], [e]) if b.0 < e.0 => Ok(format!("{}{block}{}", &existing[..b.0], &existing[e.1..])),
        (b, e) if b.len() > 1 || e.len() > 1 => Err(MergeError::Duplicate),
        _ => Err(MergeError::Unbalanced),
    }
}

/// What the launcher cleanup does with the files of `~/.local/bin`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherPlan {
    pub remove: Vec<String>,
    pub keep: Vec<String>,
}

/// `kawaro` and `kawarod` in `local` are stale when a binary of the same
/// name is installed; with none installed they are kept. Other names are
/// never touched.
pub fn stale_launchers(local: &[&str], installed: &[&str]) -> LauncherPlan {
    let mut plan = LauncherPlan {
        remove: vec![],
        keep: vec![],
    };
    for name in local.iter().filter(|n| matches!(**n, "kawaro" | "kawarod")) {
        let list = if installed.contains(name) {
            &mut plan.remove
        } else {
            &mut plan.keep
        };
        list.push((*name).to_string());
    }
    plan
}

/// What doctor knows about the drop key, for the `drop key state:` line.
#[derive(Debug)]
pub struct StateInput {
    pub key: String,
    pub bind: BindState,
    /// A bind of someone else sits on the key.
    pub foreign: bool,
    /// The portal shortcut is remembered as denied or switched off.
    pub denied: bool,
    /// The key of a registered portal fallback, when there is one.
    pub fallback: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Native(String),
    Fallback(String),
    Conflict(String),
    Denied,
}

/// Precedence: conflict, then native, then fallback, then denied. No signal
/// gives no state.
pub fn drop_key_state(i: &StateInput) -> Option<State> {
    if i.foreign {
        Some(State::Conflict(i.key.clone()))
    } else if matches!(i.bind, BindState::Present(_)) {
        Some(State::Native(i.key.clone()))
    } else if let Some(k) = &i.fallback {
        Some(State::Fallback(k.clone()))
    } else if i.denied {
        Some(State::Denied)
    } else {
        None
    }
}

/// The English value of a state.
pub fn state_text(s: &State) -> String {
    state_in(Lang::En, s)
}

/// A state in `lang`, without the line prefix.
pub fn state_in(lang: Lang, s: &State) -> String {
    let (k, key) = match s {
        State::Native(k) => ("state.native", k.as_str()),
        State::Fallback(k) => ("state.fallback", k.as_str()),
        State::Conflict(k) => ("state.conflict", k.as_str()),
        State::Denied => ("state.denied", ""),
    };
    message(lang, k).replace("{key}", key)
}

const MESSAGES: [(&str, &str, &str); 12] = [
    ("state.prefix", "drop key state:", "حالة مفتاح الإفلات:"),
    ("state.native", "native {key} active", "{key} الأصلي فعّال"),
    (
        "state.fallback",
        "fallback {key} active",
        "البديل {key} فعّال",
    ),
    (
        "state.conflict",
        "conflict: {key} taken",
        "تعارض: {key} مستخدم",
    ),
    ("state.denied", "unavailable (denied)", "غير متاح (مرفوض)"),
    (
        "repair.wrote",
        "repair: wrote 1 block",
        "الإصلاح: كُتبت كتلة واحدة",
    ),
    (
        "repair.updated",
        "repair: updated 1 block",
        "الإصلاح: حُدّثت كتلة واحدة",
    ),
    (
        "repair.nothing",
        "repair: nothing to do",
        "الإصلاح: لا شيء يلزم",
    ),
    (
        "repair.conflict",
        "conflict: {key} taken",
        "تعارض: {key} مستخدم",
    ),
    (
        "repair.not_hyprland",
        "repair: not a Hyprland session",
        "الإصلاح: الجلسة ليست Hyprland",
    ),
    ("launcher.removed", "removed: {path}", "حُذف: {path}"),
    (
        "launcher.keep",
        "keep: no installed kawaro found",
        "أُبقي: لا يوجد kawaro مثبّت",
    ),
];

/// The keys of the message table in `lang`.
pub fn message_keys(lang: Lang) -> Vec<&'static str> {
    let _ = lang;
    MESSAGES.iter().map(|m| m.0).collect()
}

/// One message; `{key}` and `{path}` are filled by the caller.
pub fn message(lang: Lang, key: &str) -> &'static str {
    MESSAGES
        .iter()
        .find(|m| m.0 == key)
        .map(|m| if lang == Lang::Ar { m.2 } else { m.1 })
        .unwrap_or("")
}

fn is_owned(b: &serde_json::Value) -> bool {
    b["arg"].as_str().is_some_and(|a| a.contains("kawaro"))
        || b["description"]
            .as_str()
            .is_some_and(|d| d.starts_with("Kawaro"))
}

/// A bind in `hyprctl binds -j` of someone else on the drop key.
pub fn foreign_on(binds_json: &str, key: &str) -> bool {
    let want = parse_key(key).key;
    serde_json::from_str::<serde_json::Value>(binds_json)
        .ok()
        .and_then(|v| v.as_array().cloned())
        .is_some_and(|binds| {
            binds.iter().any(|b| {
                b["key"]
                    .as_str()
                    .is_some_and(|k| k.eq_ignore_ascii_case(&want))
                    && !is_owned(b)
            })
        })
}

/// The portal fallback: a `global` bind whose argument ends with `:drop`,
/// named from its modmask and key (`Super+Z`).
pub fn fallback_key(binds_json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(binds_json).ok()?;
    let b = v.as_array()?.iter().find(|b| {
        b["dispatcher"].as_str() == Some("global")
            && b["arg"].as_str().is_some_and(|a| a.ends_with(":drop"))
    })?;
    let mask = b["modmask"].as_u64().unwrap_or(0);
    let mut parts: Vec<String> = [(64, "Super"), (4, "Ctrl"), (8, "Alt"), (1, "Shift")]
        .iter()
        .filter(|(bit, _)| mask & bit != 0)
        .map(|(_, n)| (*n).to_string())
        .collect();
    let key = b["key"].as_str()?;
    parts.push(if key.chars().count() == 1 {
        key.to_uppercase()
    } else {
        key.to_string()
    });
    Some(parts.join("+"))
}
