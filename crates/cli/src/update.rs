//! `kawaro update` (S-054c, ADR 0006): check, download, verify and install
//! from a terminal, with stable exit codes.
//!
//! Every rule is in `kawaro_core::update`; this file only reads the
//! arguments, calls the flow and prints. It uses `kawaro_core`, `clap` and
//! `std` only, because `tests/s054c_update.rs` compiles it on its own.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Parser;
use kawaro_core::config::Config;
use kawaro_core::i18n::Lang;
use kawaro_core::update::apply::{
    CommandRunner, Facts as SystemFacts, Plan, apply_now, linux_plan,
};
use kawaro_core::update::fetch::Fetcher;
use kawaro_core::update::flow::{
    CheckEnv, CheckOutcome, CheckState, INTERVAL_SECS, Update, check_with, download_verified,
};
use kawaro_core::update::policy::Platform;
use kawaro_core::update::result::{Outcome, read_result};
use kawaro_core::update::version::is_newer;
use kawaro_core::update::{Channel, UpdateError, effective_channel};

/// `0`: up to date, or the install finished.
const EXIT_OK: i32 = 0;
/// The install ran and failed.
const EXIT_FAILED: i32 = 1;
const EXIT_USAGE: i32 = 2;
const EXIT_OFFLINE: i32 = 3;
/// No curl program.
const EXIT_UNAVAILABLE: i32 = 4;
const EXIT_REJECTED: i32 = 5;
/// A newer version exists and is not installed.
const EXIT_AVAILABLE: i32 = 10;

/// What the command knows about the machine, passed in so a test can set it.
pub struct Facts<'a> {
    /// Picks the install plan.
    pub system: SystemFacts,
    /// The running version.
    pub installed: String,
    /// The trusted minisign public keys.
    pub keys: Vec<String>,
    pub platform: Platform,
    /// Holds `config.toml` and `update-result.json`.
    pub config_dir: PathBuf,
    /// Packages go to `<cache_dir>/updates/<version>/<name>`.
    pub cache_dir: PathBuf,
    /// The program the privileged step runs.
    pub exe: PathBuf,
    /// Runs the installer for `apply_now`.
    pub runner: &'a dyn CommandRunner,
}

/// Check for an update, download and verify it, and install it.
#[derive(Parser)]
#[command(
    name = "update",
    bin_name = "kawaro update",
    disable_version_flag = true
)]
struct Args {
    /// Only check: print the current and latest version and the status.
    #[arg(long, conflicts_with_all = ["apply", "status"])]
    check: bool,
    /// Check, download, verify and install the update.
    #[arg(long, conflicts_with = "status")]
    apply: bool,
    /// Print the state of the last download or install, without the network.
    #[arg(long)]
    status: bool,
    /// Follow this release line for this run instead of the configured one.
    #[arg(long, value_name = "stable|beta")]
    channel: Option<String>,
}

/// Every word and message the command prints.
#[derive(Clone, Copy, Debug)]
enum Msg {
    Current,
    Latest,
    Status,
    Unknown,
    UpToDate,
    Available,
    Offline,
    Unavailable,
    Rejected,
    None,
    Ready,
    Failed,
    Downloaded,
    Verified,
    Installed,
    InstallFailed,
    Manual,
    UnknownChannel,
}

fn text(lang: Lang, msg: Msg) -> &'static str {
    match (lang, msg) {
        (Lang::En, Msg::Current) => "current",
        (Lang::En, Msg::Latest) => "latest",
        (Lang::En, Msg::Status) => "status",
        (Lang::En, Msg::Unknown) => "unknown",
        (Lang::En, Msg::UpToDate) => "up-to-date",
        (Lang::En, Msg::Available) => "update-available",
        (Lang::En, Msg::Offline) => "offline",
        (Lang::En, Msg::Unavailable) => "unavailable",
        (Lang::En, Msg::Rejected) => "rejected",
        (Lang::En, Msg::None) => "none",
        (Lang::En, Msg::Ready) => "ready",
        (Lang::En, Msg::Failed) => "failed",
        (Lang::En, Msg::Downloaded) => "downloaded",
        (Lang::En, Msg::Verified) => "verified sha256",
        (Lang::En, Msg::Installed) => "installed",
        (Lang::En, Msg::InstallFailed) => "install failed",
        (Lang::En, Msg::Manual) => "install it by hand",
        (Lang::En, Msg::UnknownChannel) => "unknown channel, expected stable or beta",
        (Lang::Ar, Msg::Current) => "الحالي",
        (Lang::Ar, Msg::Latest) => "الأحدث",
        (Lang::Ar, Msg::Status) => "الحالة",
        (Lang::Ar, Msg::Unknown) => "غير معروف",
        (Lang::Ar, Msg::UpToDate) => "محدّث",
        (Lang::Ar, Msg::Available) => "يوجد تحديث",
        (Lang::Ar, Msg::Offline) => "دون اتصال",
        (Lang::Ar, Msg::Unavailable) => "غير متاح",
        (Lang::Ar, Msg::Rejected) => "مرفوض",
        (Lang::Ar, Msg::None) => "لا شيء",
        (Lang::Ar, Msg::Ready) => "جاهز",
        (Lang::Ar, Msg::Failed) => "فشل",
        (Lang::Ar, Msg::Downloaded) => "نُزّل",
        (Lang::Ar, Msg::Verified) => "تحقّق sha256",
        (Lang::Ar, Msg::Installed) => "ثُبّت",
        (Lang::Ar, Msg::InstallFailed) => "فشل التثبيت",
        (Lang::Ar, Msg::Manual) => "ثبّته يدويًا",
        (Lang::Ar, Msg::UnknownChannel) => "قناة غير معروفة، المتاح stable أو beta",
    }
}

/// Why the update was refused, in the user's language.
fn reason(lang: Lang, e: &UpdateError) -> String {
    if lang == Lang::En {
        return e.to_string();
    }
    match e {
        UpdateError::BadVersion => "ليس رقم إصدار صالحًا",
        UpdateError::UnsupportedSchema => "صيغة بيان غير مدعومة",
        UpdateError::InvalidJson => "البيان غير صالح",
        UpdateError::BadSha256 => "بصمة sha256 غير صالحة",
        UpdateError::BadSize => "الحجم صفر",
        UpdateError::BadKind => "نوع الحزمة ليس msi أو rpm أو deb",
        UpdateError::ChannelMismatch => "البيان لقناة أخرى",
        UpdateError::BadUrl => "رابط حزمة خارج تنزيلات الإصدارات",
        UpdateError::BadSignature => "لا يوجد مفتاح موثوق يتحقق من التوقيع",
        UpdateError::BadKey => "ليس مفتاح minisign عامًا",
        UpdateError::Offline => "الشبكة غير متاحة",
        UpdateError::Unavailable => "لم يوجد برنامج curl",
        UpdateError::Http(code) => return format!("أجاب الخادم HTTP {code}"),
        UpdateError::HashMismatch => "الحزمة لا تطابق بصمتها الموقّعة",
        UpdateError::SizeMismatch => "الحزمة لا تطابق حجمها الموقّع",
        UpdateError::NoAsset => "لا توجد حزمة لهذا النظام في البيان",
        UpdateError::Io => "تعذّرت قراءة ملف أو كتابته",
    }
    .to_string()
}

/// The exit code for a download that failed.
fn download_code(e: &UpdateError) -> i32 {
    match e {
        UpdateError::Offline => EXIT_OFFLINE,
        UpdateError::Unavailable => EXIT_UNAVAILABLE,
        _ => EXIT_REJECTED,
    }
}

/// `kawaro update ARGS`: the exit code of AC2.
pub fn run(
    args: &[String],
    fetcher: &dyn Fetcher,
    facts: &Facts,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let words = std::iter::once("update").chain(args.iter().map(String::as_str));
    let args = match Args::try_parse_from(words) {
        Ok(args) => args,
        Err(e) => {
            let code = e.exit_code();
            let _ = if code == EXIT_OK {
                write!(out, "{}", e.render())
            } else {
                write!(err, "{}", e.render())
            };
            return code;
        }
    };
    let (config, _) = Config::load_from(&facts.config_dir.join("config.toml"));
    let lang = config.lang();
    let channel = match args.channel.as_deref() {
        None => effective_channel(config.configured_update_channel(), &facts.installed),
        Some("stable") => Channel::Stable,
        Some("beta") => Channel::Beta,
        Some(other) => {
            let _ = writeln!(err, "{}: {other}", text(lang, Msg::UnknownChannel));
            return EXIT_USAGE;
        }
    };
    if args.status {
        let _ = writeln!(out, "{}", status_line(facts, lang));
        return EXIT_OK;
    }
    let mut followed = config;
    followed.update_channel = channel;
    let update = match check(fetcher, facts, &followed, lang, out, err) {
        Ok(update) => update,
        Err(code) => return code,
    };
    if args.check {
        return EXIT_AVAILABLE;
    }
    install(&update, fetcher, facts, lang, args.apply, out, err)
}

/// Runs the manual check and prints its three lines: the newer version, or
/// the exit code that ends the run.
fn check(
    fetcher: &dyn Fetcher,
    facts: &Facts,
    config: &Config,
    lang: Lang,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<Update, i32> {
    let scratch = facts
        .cache_dir
        .join(format!("check-{}", std::process::id()));
    let _ = fs::create_dir_all(&scratch);
    let keys: Vec<&str> = facts.keys.iter().map(String::as_str).collect();
    let env = CheckEnv {
        installed: &facts.installed,
        keys: &keys,
        platform: facts.platform.clone(),
        interval_secs: INTERVAL_SECS,
        scratch: &scratch,
    };
    let mut state = CheckState::default();
    let outcome = check_with(&env, config, &mut state, fetcher, 0, true);
    let _ = fs::remove_dir_all(&scratch);
    // Leaves no empty cache folder behind; fails harmlessly when it holds files.
    let _ = fs::remove_dir(&facts.cache_dir);
    let (status, code) = match &outcome {
        CheckOutcome::UpToDate => (Msg::UpToDate, EXIT_OK),
        CheckOutcome::Available(_) => (Msg::Available, EXIT_AVAILABLE),
        CheckOutcome::Offline => (Msg::Offline, EXIT_OFFLINE),
        CheckOutcome::Unavailable => (Msg::Unavailable, EXIT_UNAVAILABLE),
        _ => (Msg::Rejected, EXIT_REJECTED),
    };
    let latest = state
        .last_seen
        .as_deref()
        .unwrap_or(text(lang, Msg::Unknown));
    let _ = writeln!(out, "{} {}", text(lang, Msg::Current), facts.installed);
    let _ = writeln!(out, "{} {latest}", text(lang, Msg::Latest));
    let _ = writeln!(out, "{} {}", text(lang, Msg::Status), text(lang, status));
    if let CheckOutcome::Rejected(e) = &outcome {
        let _ = writeln!(err, "{}: {}", text(lang, Msg::Rejected), reason(lang, e));
    }
    match outcome {
        CheckOutcome::Available(update) => Ok(update),
        _ => Err(code),
    }
}

/// Downloads and verifies `update`, then installs it when `apply` is set.
fn install(
    update: &Update,
    fetcher: &dyn Fetcher,
    facts: &Facts,
    lang: Lang,
    apply: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let dir = facts.cache_dir.join("updates").join(&update.version);
    let package = match download_verified(update, &dir, fetcher) {
        Ok(package) => package,
        Err(e) => {
            let _ = writeln!(err, "{}: {}", text(lang, Msg::Rejected), reason(lang, &e));
            return download_code(&e);
        }
    };
    let sha256 = &update.asset.sha256;
    let _ = writeln!(out, "{} {}", text(lang, Msg::Downloaded), update.asset.name);
    let _ = writeln!(out, "{} {}", text(lang, Msg::Verified), &sha256[..12]);
    if !apply {
        return EXIT_AVAILABLE;
    }
    let plan = linux_plan(&facts.system);
    match apply_plan(&plan, facts, package.path(), sha256, lang) {
        Ok(()) => {
            let _ = writeln!(out, "{} {}", text(lang, Msg::Installed), update.version);
            EXIT_OK
        }
        Err(line) => {
            let _ = writeln!(err, "{line}");
            EXIT_FAILED
        }
    }
}

/// Installs through `apply_now`; a plan that runs nothing gives the line
/// that tells the user how to install by hand.
fn apply_plan(
    plan: &Plan,
    facts: &Facts,
    path: &Path,
    sha256: &str,
    lang: Lang,
) -> Result<(), String> {
    match plan {
        Plan::ShowCommand(how) | Plan::ShowRelease(how) => Err(format!(
            "{}: {}",
            text(lang, Msg::Manual),
            how.replace("{file}", &path.to_string_lossy())
        )),
        _ => apply_now(plan, &facts.exe, path, sha256, facts.runner)
            .map_err(|e| format!("{}: {}", text(lang, Msg::InstallFailed), reason(lang, &e))),
    }
}

/// `status none`, `status ready <v>` or `status failed <v>`, from local files
/// only: a failed install in `update-result.json`, else a verified package of
/// a newer version in `<cache_dir>/updates/`.
fn status_line(facts: &Facts, lang: Lang) -> String {
    let label = text(lang, Msg::Status);
    if let Some(r) = read_result(&facts.config_dir)
        && let Outcome::Failed(_) = r.outcome
    {
        return format!("{label} {} {}", text(lang, Msg::Failed), r.version);
    }
    match ready_version(&facts.cache_dir.join("updates"), &facts.installed) {
        Some(v) => format!("{label} {} {v}", text(lang, Msg::Ready)),
        None => format!("{label} {}", text(lang, Msg::None)),
    }
}

/// The version of a verified package newer than `installed`. A package is
/// verified once it lost its `.part` name (`download_verified` renames it
/// only after its size and SHA-256 matched).
fn ready_version(updates: &Path, installed: &str) -> Option<String> {
    fs::read_dir(updates)
        .ok()?
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|v| is_newer(installed, v).unwrap_or(false))
        .find(|v| has_package(&updates.join(v)))
}

fn has_package(dir: &Path) -> bool {
    fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|e| {
            let is_file = e.file_type().is_ok_and(|t| t.is_file());
            is_file && !e.file_name().to_string_lossy().ends_with(".part")
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasons_in_both_languages() {
        let e = UpdateError::HashMismatch;
        assert_eq!(
            reason(Lang::En, &e),
            "the package does not match its signed sha256"
        );
        assert_eq!(reason(Lang::Ar, &e), "الحزمة لا تطابق بصمتها الموقّعة");
        assert_eq!(
            reason(Lang::Ar, &UpdateError::Http(404)),
            "أجاب الخادم HTTP 404"
        );
    }

    #[test]
    fn download_failures_keep_their_exit_codes() {
        assert_eq!(download_code(&UpdateError::Offline), EXIT_OFFLINE);
        assert_eq!(download_code(&UpdateError::Unavailable), EXIT_UNAVAILABLE);
        assert_eq!(download_code(&UpdateError::HashMismatch), EXIT_REJECTED);
    }

    struct NoRun;

    impl CommandRunner for NoRun {
        fn run(&self, _: &[String]) -> Result<i32, UpdateError> {
            panic!("a show plan runs nothing");
        }
    }

    #[test]
    fn show_plans_tell_how_to_install_by_hand() {
        let facts = Facts {
            system: SystemFacts::default(),
            installed: "0.3.3".into(),
            keys: Vec::new(),
            platform: Platform {
                os: "linux".into(),
                arch: "x86_64".into(),
                pkg: "rpm".into(),
            },
            config_dir: PathBuf::new(),
            cache_dir: PathBuf::new(),
            exe: PathBuf::from("/usr/bin/kawaro"),
            runner: &NoRun,
        };
        let file = Path::new("/c/updates/0.4.0/s.rpm");
        let show = Plan::ShowCommand("sudo dnf install -y {file}".into());
        assert_eq!(
            apply_plan(&show, &facts, file, "ab", Lang::En),
            Err("install it by hand: sudo dnf install -y /c/updates/0.4.0/s.rpm".into())
        );
        let page = Plan::ShowRelease("https://example.org/r".into());
        assert_eq!(
            apply_plan(&page, &facts, file, "ab", Lang::Ar),
            Err("ثبّته يدويًا: https://example.org/r".into())
        );
    }

    #[test]
    fn ready_needs_a_finished_package_of_a_newer_version() {
        let t = tempfile::tempdir().unwrap();
        let updates = t.path().join("updates");
        let add = |v: &str, name: &str| {
            fs::create_dir_all(updates.join(v)).unwrap();
            fs::write(updates.join(v).join(name), b"x").unwrap();
        };
        assert_eq!(ready_version(&updates, "0.3.3"), None);
        add("0.3.0", "old.rpm");
        add("0.4.0", "new.rpm.part");
        fs::create_dir_all(updates.join("0.5.0").join("sub")).unwrap();
        add("notes", "x.rpm");
        assert_eq!(ready_version(&updates, "0.3.3"), None);
        add("0.4.0", "new.rpm");
        assert_eq!(ready_version(&updates, "0.3.3").as_deref(), Some("0.4.0"));
        assert_eq!(ready_version(&updates, "0.4.0"), None);
    }
}
