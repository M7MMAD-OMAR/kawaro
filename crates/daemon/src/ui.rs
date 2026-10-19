//! Bridge to the hive window, which lives on the main thread with the Slint
//! event loop. Other threads post work with `invoke_from_event_loop`.

use sbarhive_core::ActionDef;
use sbarhive_core::i18n::Lang;
use sbarhive_ui::{Hive, HiveCell};
use std::cell::RefCell;

thread_local! {
    static HIVE: RefCell<Option<Hive>> = const { RefCell::new(None) };
}

pub fn install(hive: Hive) {
    HIVE.with(|h| *h.borrow_mut() = Some(hive));
}

pub fn cell(a: &ActionDef, lang: Lang) -> HiveCell {
    HiveCell {
        id: a.id.clone(),
        code: a.code.get(lang).to_string(),
        label: a.label.get(lang).to_string(),
        ring: a.ring,
    }
}

pub fn present(centre: String, cells: Vec<HiveCell>, token: Option<String>) {
    let _ = slint::invoke_from_event_loop(move || {
        HIVE.with(|h| {
            if let Some(hive) = h.borrow().as_ref()
                && let Err(e) = hive.present(&centre, cells, token)
            {
                eprintln!("sbarhived: cannot show the hive: {e}");
            }
        })
    });
}

pub fn quit() {
    let _ = slint::invoke_from_event_loop(|| {
        let _ = slint::quit_event_loop();
    });
}
