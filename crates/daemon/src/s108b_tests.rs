//! S-108 part B (backlog/S-108-s096b-hive-cells.md), the request click on
//! the real window with an injected opener (AC5 interface half).
//!
//! Hooks these tests pin:
//! - `ui::wire_choose(hive: &Hive, state: &Shared, queue: &Arc<Queue>,
//!   opener: Rc<dyn Fn(&str)>)` installs the `on_choose` handler; the daemon
//!   start up calls it with the real opener (`open_url_command` through
//!   `notify::open_folder`), the tests with a counting spy.
//! - `kawaro_ui::Hive::click_cell(&self, id: &str) -> bool` and
//!   `Hive::return_on_cell(&self, id: &str) -> bool`, behind the `testing-hooks`
//!   feature and `#[doc(hidden)]`: they send the real pointer events (move,
//!   press, release) or a pointer move plus the Return key to the window at
//!   the cell's centre, and return whether the cell exists. The daemon's
//!   dev-dependency on `kawaro-ui` enables that feature.
//! - `kawaro_ui::Hive::set_notes` (see `crates/ui/src/s108b_tests.rs`).

use crate::plan::plan;
use crate::state::{Shared, State};
use crate::ui::wire_choose;
use kawaro_core::config::Config;
use kawaro_core::i18n::Lang;
use kawaro_core::queue::Queue;
use kawaro_core::unsupported::request_url;
use kawaro_ui::Hive;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex, mpsc};

fn wired(spy: &Rc<RefCell<Vec<String>>>) -> Hive {
    i_slint_backend_testing::init_no_event_loop();
    let hive = Hive::new(Lang::En).unwrap();
    let mut st = State::new(Config::default(), &[]);
    st.available = ["image".to_string()].into();
    let shared: Shared = Arc::new(Mutex::new(st));
    let (tx, _rx) = mpsc::channel();
    let queue = Arc::new(Queue::new(kawaro_engines::all(), 1, tx));
    let s = spy.clone();
    wire_choose(
        &hive,
        &shared,
        &queue,
        Rc::new(move |url: &str| s.borrow_mut().push(url.to_string())),
    );
    let files = vec![PathBuf::from("/x/a.png"), PathBuf::from("/x/b.xyz")];
    let st = shared.lock().unwrap();
    let outcome = st.classify(&files);
    let p = plan(&outcome, &files, &st);
    drop(st);
    hive.present(&p.centre, p.cells, None).unwrap();
    hive
}

#[test]
fn ac5_showing_the_hive_opens_nothing_and_a_click_on_req_opens_once() {
    let spy = Rc::new(RefCell::new(Vec::new()));
    let hive = wired(&spy);
    assert_eq!(spy.borrow().len(), 0, "0 calls on show");
    assert!(hive.click_cell("request:xyz"));
    assert_eq!(*spy.borrow(), [request_url("xyz")], "1 call on click");
}

#[test]
fn ac5_return_on_req_opens_again_and_a_skipped_cell_adds_nothing() {
    let spy = Rc::new(RefCell::new(Vec::new()));
    let hive = wired(&spy);
    assert!(hive.click_cell("skipped:0"));
    assert_eq!(spy.borrow().len(), 0, "a greyed cell opens nothing");
    assert!(hive.click_cell("request:xyz"));
    assert_eq!(spy.borrow().len(), 1);
    // The request closes the hive; show it again and use the keyboard.
    let hive2 = wired(&spy);
    assert!(hive2.return_on_cell("request:xyz"));
    assert_eq!(spy.borrow().len(), 2, "Return on the cell is one more call");
    assert_eq!(spy.borrow()[1], request_url("xyz"));
    assert!(hive2.click_cell("skipped:0"));
    assert_eq!(spy.borrow().len(), 2);
}
