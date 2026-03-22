//! Combined test harness for Phase 11: hydra-reflexive + hydra-morphic + hydra-persona.

use hydra_morphic::{MorphicEventKind, MorphicIdentity};
use hydra_persona::{Persona, PersonaBlend, PersonaRegistry};
use hydra_reflexive::{
    CapabilitySource, ModificationKind, ModificationProposal, SafeModifier, SelfModel, SelfSnapshot,
};

fn main() {
    println!("=== Phase 11 Combined Test Harness ===\n");
    let mut p = 0u32;
    let mut f = 0u32;

    // Reflexive
    t("reflexive: bootstrap", &mut p, &mut f, || {
        let m = SelfModel::bootstrap_layer1();
        assert_eq!(m.capabilities.len(), 5);
        assert_eq!(m.total_ever, 5);
    });
    t("reflexive: total_ever monotonic", &mut p, &mut f, || {
        let mut m = SelfModel::bootstrap_layer1();
        let before = m.total_ever;
        m.add_capability(
            "x",
            CapabilitySource::Skill {
                skill_id: "s".into(),
            },
        )
        .unwrap();
        assert!(m.total_ever > before);
    });
    t("reflexive: safe modification", &mut p, &mut f, || {
        let mut m = SelfModel::bootstrap_layer1();
        let mut sm = SafeModifier::new();
        let pr = ModificationProposal::new(ModificationKind::AddCapability, "add", "root");
        sm.apply(&mut m, &pr, |m| {
            m.add_capability(
                "n",
                CapabilitySource::Skill {
                    skill_id: "s".into(),
                },
            )
        })
        .unwrap();
        assert_eq!(m.capabilities.len(), 6);
    });
    t("reflexive: rollback", &mut p, &mut f, || {
        let mut m = SelfModel::bootstrap_layer1();
        let mut sm = SafeModifier::new();
        let pr = ModificationProposal::new(ModificationKind::AddCapability, "rb", "root");
        sm.apply(&mut m, &pr, |m| {
            m.add_capability(
                "tmp",
                CapabilitySource::Skill {
                    skill_id: "s".into(),
                },
            )
        })
        .unwrap();
        sm.rollback_last(&mut m).unwrap();
        assert_eq!(m.capabilities.len(), 5);
    });
    t("reflexive: snapshot roundtrip", &mut p, &mut f, || {
        let m = SelfModel::bootstrap_layer1();
        let s = SelfSnapshot::capture(&m).unwrap();
        let r = s.restore().unwrap();
        assert_eq!(r.capabilities.len(), m.capabilities.len());
        assert_eq!(r.total_ever, m.total_ever);
    });

    // Morphic
    t("morphic: genesis", &mut p, &mut f, || {
        let id = MorphicIdentity::genesis();
        assert_eq!(id.depth(), 0);
        assert!(id.history.is_empty());
    });
    t("morphic: deepen", &mut p, &mut f, || {
        let mut id = MorphicIdentity::genesis();
        id.record_event(MorphicEventKind::CapabilityAdded { name: "c".into() })
            .unwrap();
        assert_eq!(id.depth(), 1);
    });
    t("morphic: restart", &mut p, &mut f, || {
        let mut id = MorphicIdentity::genesis();
        id.record_restart().unwrap();
        assert!(id.signature.restart_count > 0);
    });
    t("morphic: distance zero to clone", &mut p, &mut f, || {
        let id = MorphicIdentity::genesis();
        assert!(id.signature.distance(&id.clone().signature).abs() < 1e-10);
    });
    t("morphic: distance grows", &mut p, &mut f, || {
        let mut a = MorphicIdentity::genesis();
        let b = a.clone();
        for i in 0..5 {
            a.record_event(MorphicEventKind::CapabilityAdded {
                name: format!("c{i}"),
            })
            .unwrap();
        }
        assert!(a.signature.distance(&b.signature) > 0.0);
    });
    t("morphic: events recorded", &mut p, &mut f, || {
        let mut id = MorphicIdentity::genesis();
        id.record_event(MorphicEventKind::SkillLoaded {
            skill_id: "s1".into(),
        })
        .unwrap();
        id.record_event(MorphicEventKind::SisterConnected {
            sister_name: "mem".into(),
        })
        .unwrap();
        assert_eq!(id.history.len(), 2);
    });

    // Persona
    t("persona: core pre-loaded", &mut p, &mut f, || {
        assert!(PersonaRegistry::new().get("hydra-core").is_some());
    });
    t("persona: activate", &mut p, &mut f, || {
        let mut r = PersonaRegistry::new();
        r.activate("hydra-core").unwrap();
        assert!(r.active_voice().is_some());
    });
    t("persona: blend", &mut p, &mut f, || {
        let mut r = PersonaRegistry::new();
        r.register(Persona::security_analyst_persona()).unwrap();
        let b = PersonaBlend::weighted(vec![
            ("hydra-core".into(), 0.6),
            ("security-analyst".into(), 0.4),
        ])
        .unwrap();
        r.set_blend(b).unwrap();
        assert!(r.active_voice().unwrap().is_active());
    });
    t("persona: invalid weights", &mut p, &mut f, || {
        assert!(PersonaBlend::weighted(vec![("a".into(), 0.5), ("b".into(), 0.6)]).is_err());
    });
    t("persona: unregistered fails", &mut p, &mut f, || {
        assert!(PersonaRegistry::new().activate("nope").is_err());
    });

    // Integration
    t("integ: skill load pipeline", &mut p, &mut f, || {
        let mut m = SelfModel::bootstrap_layer1();
        let mut sm = SafeModifier::new();
        let mut id = MorphicIdentity::genesis();
        let mut reg = PersonaRegistry::new();
        let pr = ModificationProposal::new(ModificationKind::AddCapability, "git", "test");
        sm.apply(&mut m, &pr, |m| {
            m.add_capability(
                "git-skill",
                CapabilitySource::Skill {
                    skill_id: "git".into(),
                },
            )
        })
        .unwrap();
        id.record_event(MorphicEventKind::SkillLoaded {
            skill_id: "git".into(),
        })
        .unwrap();
        reg.activate("hydra-core").unwrap();
        assert!(m.get("git-skill").is_some());
        assert_eq!(id.depth(), 1);
        assert!(reg.active_voice().is_some());
    });
    t(
        "integ: rollback preserves morphic depth",
        &mut p,
        &mut f,
        || {
            let mut m = SelfModel::bootstrap_layer1();
            let mut sm = SafeModifier::new();
            let mut id = MorphicIdentity::genesis();
            let pr = ModificationProposal::new(ModificationKind::AddCapability, "t", "test");
            sm.apply(&mut m, &pr, |m| {
                m.add_capability(
                    "tmp",
                    CapabilitySource::Skill {
                        skill_id: "t".into(),
                    },
                )
            })
            .unwrap();
            id.record_event(MorphicEventKind::SelfModificationApplied {
                description: "added".into(),
            })
            .unwrap();
            let d = id.depth();
            sm.rollback_last(&mut m).unwrap();
            id.record_event(MorphicEventKind::SelfModificationRolledBack {
                description: "rolled back".into(),
            })
            .unwrap();
            assert!(id.depth() > d);
        },
    );
    t("integ: architect+security blend", &mut p, &mut f, || {
        let mut r = PersonaRegistry::new();
        r.register(Persona::security_analyst_persona()).unwrap();
        r.register(Persona::software_architect_persona()).unwrap();
        let b = PersonaBlend::weighted(vec![
            ("security-analyst".into(), 0.5),
            ("software-architect".into(), 0.5),
        ])
        .unwrap();
        r.set_blend(b).unwrap();
        assert!(!r.active_voice().unwrap().voice.vocabulary.is_empty());
    });
    t("integ: summaries non-empty", &mut p, &mut f, || {
        let m = SelfModel::bootstrap_layer1();
        let id = MorphicIdentity::genesis();
        let mut r = PersonaRegistry::new();
        r.activate("hydra-core").unwrap();
        assert!(!m.summary().is_empty());
        assert!(!id.summary().is_empty());
        assert!(!r.active_voice().unwrap().summary().is_empty());
    });
    t("integ: signature never decreases", &mut p, &mut f, || {
        let mut id = MorphicIdentity::genesis();
        let mut last = id.depth();
        for i in 0..10 {
            id.record_event(MorphicEventKind::GenomeRecorded {
                entry_id: format!("g{i}"),
            })
            .unwrap();
            assert!(id.depth() > last);
            last = id.depth();
        }
    });
    t("integ: snapshot preserves state", &mut p, &mut f, || {
        let mut m = SelfModel::bootstrap_layer1();
        let s = SelfSnapshot::capture(&m).unwrap();
        m.add_capability(
            "post",
            CapabilitySource::Skill {
                skill_id: "s".into(),
            },
        )
        .unwrap();
        assert_eq!(m.capabilities.len(), 6);
        assert_eq!(s.restore().unwrap().capabilities.len(), 5);
    });

    println!("\n=== Results: {p} passed, {f} failed ===");
    if f > 0 {
        std::process::exit(1);
    }
}

fn t(name: &str, passed: &mut u32, failed: &mut u32, func: impl FnOnce()) {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(func)) {
        Ok(()) => {
            println!("  PASS: {name}");
            *passed += 1;
        }
        Err(e) => {
            let msg = e
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| e.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown".to_string());
            println!("  FAIL: {name} — {msg}");
            *failed += 1;
        }
    }
}
