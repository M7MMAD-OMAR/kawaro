//! Opens the settings window with the current settings and prints what Save
//! would store. Writes nothing.
//!
//!     cargo run -p kawaro-ui --example settings_demo -- [--lang ar|en]

use kawaro_core::config::Config;
use kawaro_ui::Lang;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let lang = args
        .windows(2)
        .find(|w| w[0] == "--lang")
        .map(|w| Lang::parse(&w[1]))
        .unwrap_or(Lang::En);
    kawaro_ui::init()?;
    let (config, warning) = Config::load();
    if let Some(w) = warning {
        eprintln!("warning: {w}");
    }
    let win = kawaro_ui::SettingsWindow::new(lang)?;
    win.on_save(|c| {
        println!("save: {c:?}");
        slint::quit_event_loop().ok();
    });
    win.open(&config)?;
    slint::run_event_loop()?;
    Ok(())
}
