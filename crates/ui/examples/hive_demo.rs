//! Shows the hive with sample image actions and prints the chosen action id,
//! or "dismissed".
//!
//!     cargo run -p kawaro-ui --example hive_demo -- --lang ar
//!
//! Passes `XDG_ACTIVATION_TOKEN` to the compositor when it is set.

mod common;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let lang = common::lang_from_args(&args);
    kawaro_ui::init()?;
    let hive = kawaro_ui::Hive::new(lang)?;
    hive.on_choose(|id| {
        println!("{id}");
        slint::quit_event_loop().ok();
    });
    hive.on_dismiss(|| {
        println!("dismissed");
        slint::quit_event_loop().ok();
    });
    let token = std::env::var("XDG_ACTIVATION_TOKEN").ok();
    hive.present(
        common::centre_text(lang),
        common::image_actions(lang),
        token,
    )?;
    slint::run_event_loop_until_quit()?;
    Ok(())
}
