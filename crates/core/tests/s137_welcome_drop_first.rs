//! S-137 AC1: the tutorial always teaches the default, using the live trigger.
use kawaro_core::i18n::Lang;
use kawaro_core::welcome::{self, Registration};

#[test]
fn ac1_en_ar_registration_matrix_teaches_drop_before_choice() {
    for lang in [Lang::En, Lang::Ar] {
        for reg in [
            Registration::RightDrag,
            Registration::Native("Alt".into()),
            Registration::Fallback("Super+Z".into()),
            Registration::Unbound,
        ] {
            let c = welcome::captions(lang, &reg);
            let s = c.steps.map(|s| s.to_lowercase());
            let context = format!("{lang:?} {reg:?}: {s:?}");
            assert_eq!(s.len(), 5, "{context}");
            let all = s.join(" ");
            assert!(
                all.contains(if lang == Lang::En {
                    "default"
                } else {
                    "افتراضي"
                }),
                "explicit default tutorial label: {context}"
            );
            let words: [&[&str]; 5] = if lang == Lang::En {
                [
                    &["drag"],
                    &[],
                    &["drop"],
                    &["after", "choos", "action"],
                    &["beside", "original"],
                ]
            } else {
                [
                    &["اسحب"],
                    &[],
                    &["أفلت"],
                    &["بعد", "اختر"],
                    &["بجانب", "الأصل"],
                ]
            };
            for (row, words) in words.iter().enumerate() {
                for word in *words {
                    assert!(s[row].contains(word), "row {row} needs {word}: {context}");
                }
            }
            assert!(
                s[2].contains(if lang == Lang::En {
                    "area"
                } else {
                    "منطقة"
                }),
                "drop target is an area, not an action: {context}"
            );
            assert!(
                !s[2].contains(if lang == Lang::En { "cell" } else { "خلية" }),
                "{context}"
            );
            assert!(
                s[4].contains(if lang == Lang::En {
                    "untouched"
                } else {
                    "دون تغيير"
                }),
                "original stays untouched: {context}"
            );
            match &reg {
                Registration::RightDrag => assert!(
                    s[1].contains(if lang == Lang::En {
                        "right button"
                    } else {
                        "الزر الأيمن"
                    }),
                    "{context}"
                ),
                Registration::Native(key) | Registration::Fallback(key) => {
                    assert!(s[1].contains(&key.to_lowercase()), "{context}")
                }
                Registration::Unbound => {
                    for key in ["alt", "super", "ctrl", "shift", "win", "+"] {
                        assert!(!all.contains(key), "Unbound must not name a key: {context}");
                    }
                }
            }
            if !matches!(reg, Registration::Native(_)) {
                assert!(!s[1].contains("alt"), "never invent native Alt: {context}");
            }
            for forbidden in [
                "whole screen",
                "anywhere on the screen",
                "portal",
                "كامل الشاشة",
            ] {
                assert!(
                    !all.contains(forbidden),
                    "no universal drop claim: {context}"
                );
            }
        }
    }
}
