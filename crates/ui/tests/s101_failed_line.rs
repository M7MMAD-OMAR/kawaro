//! S-101 rework: under the red failed banner the page never claims that the
//! installation is up to date.

use kawaro_core::config::Config;
use kawaro_ui::{Lang, SettingsWindow, UpdateResult, UpdateState};

#[test]
fn the_failed_state_does_not_say_up_to_date_in_en_and_ar() {
    i_slint_backend_testing::init_no_event_loop();
    for lang in [Lang::En, Lang::Ar] {
        let w = SettingsWindow::new(lang).expect("window");
        w.open(&Config::default()).expect("open");
        w.set_update_view("0.4.2", UpdateState::Failed(String::new()), true);
        w.show_update_result(Some(UpdateResult::Failed("0.4.3".into())));
        let line = w.update_state_line();
        assert!(!line.is_empty(), "{lang:?}: a retry hint shows");
        assert!(
            !line.contains("up to date") && !line.contains("نسختك محدّثة"),
            "{lang:?}: {line}"
        );
        assert!(w.update_banner().is_some());
        assert!(w.update_check_enabled(), "{lang:?}: retry stays possible");
    }
}
