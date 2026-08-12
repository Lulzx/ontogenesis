// Ontogenesis experiment: knowledge migration across ontology changes.
//
// When a concept set is revised, every old concept is classified as preserved,
// refined, re-expressible, ambiguous, or invalidated, and two strategies are
// compared on one comparable cost ledger:
//
//   * revision + migration: carry preserved/refined knowledge forward, apply a
//     small re-expression map, re-derive only ambiguous/invalidated concepts;
//   * cold restart: discard everything and rediscover the new ontology.
//
// Both end with the identical new ontology (same description + reasoning cost);
// they differ only in transfer work. The report prints the classification, the
// two totals, the saving, and replay/held-out verification, ending with a
// deterministic machine record.
use supsearch::concept_migration::{self, migrate, MigrationKind};
use supsearch::ontology_repair::{Concept, Observation, Ontology, RepairSpec, Runner, Task};

fn obs(tokens: &[(u8, u8)]) -> Vec<Observation> {
    tokens
        .iter()
        .map(|(t, l)| Observation {
            token: *t,
            label: *l,
        })
        .collect()
}
fn task(tokens: &[(u8, u8)]) -> Task {
    Task {
        queries: tokens.iter().map(|(t, l)| (*t, *l)).collect(),
    }
}
fn con(id: u64, ext: &[u8], label: u8) -> Concept {
    Concept {
        id,
        extension: ext.iter().copied().collect(),
        label,
    }
}

fn print_report(rep: &supsearch::concept_migration::MigrationReport, title: &str) {
    println!("--- {title} ---");
    println!(
        "old concepts={} new concepts={} added={}",
        rep.old_concepts, rep.new_concepts, rep.added_concepts
    );
    for e in &rep.entries {
        println!(
            "  old#{} label={} tokens={} -> {} (transfer={})",
            e.old_id,
            e.label,
            e.tokens,
            e.kind.name(),
            e.transfer_cost
        );
    }
    println!(
        "preserved={} refined={} re-expressible={} ambiguous={} invalidated={}",
        rep.preserved, rep.refined, rep.re_expressible, rep.ambiguous, rep.invalidated
    );
    println!(
        "new_structural={} reasoning={}",
        rep.new_structural, rep.reasoning
    );
    println!(
        "migration_transfer={} cold_restart_transfer={}",
        rep.migration_transfer, rep.cold_restart_transfer
    );
    println!(
        "migration_total={} cold_restart_total={} saving={}",
        rep.migration_total, rep.cold_restart_total, rep.saving
    );
    println!(
        "replay_old_task={} held_out_answered={}",
        rep.replay_old_task, rep.held_out_answered
    );
}

fn main() {
    let mut r = Runner::new(RepairSpec::default());

    // Stage 0 (old world): four coarse concepts.
    r.add_stage(
        &obs(&[
            (0, 0), (1, 0), (2, 1), (3, 1), (4, 2), (5, 2), (8, 3), (9, 3),
        ]),
        &task(&[
            (0, 0), (1, 0), (2, 1), (3, 1), (4, 2), (5, 2), (8, 3), (9, 3),
        ]),
    );

    // Stage 1 (new world):
    //  - {0,1}->0 unchanged                      => preserved
    //  - {2,3}->1 widens to include 6,7          => refined (generalize)
    //  - {4,5}->2 re-labeled to class 9          => re-expressible
    //  - {8,9}->3 splits into labels 3 and 4     => ambiguous
    //  - new tokens 10,11->2                     => added (novel)
    r.add_stage(
        &obs(&[
            (0, 0), (1, 0), (2, 1), (3, 1), (4, 9), (5, 9),
            (6, 1), (7, 1), (8, 3), (9, 4), (10, 2), (11, 2),
        ]),
        &task(&[
            (0, 0), (1, 0), (2, 1), (3, 1), (4, 9), (5, 9),
            (6, 1), (7, 1), (8, 3), (9, 4), (10, 2), (11, 2),
        ]),
    );

    let old = r.stages[0].ontology.clone();
    let new = r.stages[1].ontology.clone();
    let old_task = task(&[(0, 0), (1, 0), (2, 1), (3, 1), (4, 2), (5, 2), (8, 3), (9, 3)]);
    let new_task = task(&[
        (0, 0), (1, 0), (2, 1), (3, 1), (4, 9), (5, 9),
        (6, 1), (7, 1), (8, 3), (9, 4), (10, 2), (11, 2),
    ]);
    let held_out = task(&[(6, 1), (7, 1), (10, 2), (11, 2)]);

    println!("ontogenesis: knowledge migration across ontology change (U7)");
    print_report(&migrate(&old, &new, &old_task, &new_task, &held_out), "Runner trajectory");

    // Invalidated: tokens leave the observed world entirely. The Runner never
    // drops accumulated observations, so we exercise this kind with a manually
    // constructed ontology pair.
    let old2 = Ontology {
        concepts: vec![con(0, &[0, 1], 0), con(1, &[2, 3], 1)],
    };
    let new2 = Ontology {
        concepts: vec![con(0, &[0, 1], 0), con(1, &[4, 5], 1)],
    };
    let ot = task(&[(0, 0), (1, 0), (2, 1), (3, 1)]);
    let nt = task(&[(0, 0), (1, 0), (4, 1), (5, 1)]);
    let rep2 = migrate(&old2, &new2, &ot, &nt, &nt);
    print_report(&rep2, "manual invalidated");
    assert_eq!(rep2.entries[1].kind, MigrationKind::Invalidated);

    // Full deterministic record from the Runner trajectory.
    println!("{}", concept_migration::machine_record(&migrate(&old, &new, &old_task, &new_task, &held_out)));
}
