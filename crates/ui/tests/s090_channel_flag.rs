//! S-090 review rework: the saved config must say whether the update channel
//! became an explicit key (S-053a). Pins `FormState::to_config` through the
//! public settings window, on the Config handed to `on_save`.

use kawaro_core::config::Config;
use kawaro_core::update::Channel;
use kawaro_ui::{Lang, SettingsWindow};
use std::cell::RefCell;
use std::rc::Rc;

fn saved_after(start: &Config, lang: Lang, edit: impl Fn(&SettingsWindow)) -> Config {
    thread_local! {
        static INIT: std::sync::Once = const { std::sync::Once::new() };
    }
    INIT.with(|once| once.call_once(i_slint_backend_testing::init_no_event_loop));
    let w = SettingsWindow::new(lang).expect("window");
    w.open(start).expect("open");
    let saved: Rc<RefCell<Option<Config>>> = Rc::default();
    let s2 = saved.clone();
    w.on_save(move |c| *s2.borrow_mut() = Some(c));
    edit(&w);
    w.press_save();
    let out = saved.borrow().clone().expect("Save calls on_save");
    out
}

#[test]
fn ac4_a_changed_channel_becomes_an_explicit_key() {
    for lang in [Lang::En, Lang::Ar] {
        let start = Config::default();
        assert!(!start.update_channel_present);
        let out = saved_after(&start, lang, |w| w.set_update_channel(Channel::Beta));
        assert_eq!(out.update_channel, Channel::Beta);
        assert!(out.update_channel_present, "{lang:?}: changed => explicit");
    }
}

#[test]
fn ac4_an_unchanged_channel_keeps_the_loaded_flag() {
    for lang in [Lang::En, Lang::Ar] {
        for present in [false, true] {
            let start = Config {
                update_channel_present: present,
                ..Config::default()
            };
            let out = saved_after(&start, lang, |_| {});
            assert_eq!(out.update_channel, Channel::Stable);
            assert_eq!(out.update_channel_present, present, "{lang:?}");
        }
    }
}

#[test]
fn ac4_an_already_explicit_channel_stays_explicit_when_changed_back() {
    let start = Config {
        update_channel: Channel::Beta,
        update_channel_present: true,
        ..Config::default()
    };
    let out = saved_after(&start, Lang::En, |w| w.set_update_channel(Channel::Stable));
    assert_eq!(out.update_channel, Channel::Stable);
    assert!(out.update_channel_present);
}
