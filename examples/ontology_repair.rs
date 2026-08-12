// Ontogenesis experiment: non-monotonic ontology repair.
//
// A behavior-bank runner accumulates world observations and a downstream task
// across stages. At each stage the ontology is rebuilt under minimum
// description length, and the transition from the previous ontology is
// classified into non-monotonic repair operations: retain / add / remove /
// split / merge / specialize / generalize / structural-replace. Each stage
// reports a comparable cost ledger and predictive replay (current-task and
// accumulated), and the whole run emits a deterministic machine-readable
// record for later audit.
use supsearch::ontology_repair::{self, Observation, RepairSpec, Runner, Task};

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

fn main() {
    let mut r = Runner::new(RepairSpec::default());

    // Stage 0: coarse two-concept world; task needs only the coarse split.
    r.add_stage(
        &obs(&[(0, 0), (1, 0), (2, 0), (3, 0), (4, 1), (5, 1), (6, 1), (7, 1)]),
        &task(&[(0, 0), (1, 0), (2, 0), (3, 0), (4, 1), (5, 1), (6, 1), (7, 1)]),
    );

    // Stage 1: the world refines; {0,1} vs {2,3} become distinct classes and
    // the task now requires the finer distinction -> split.
    r.add_stage(
        &obs(&[(0, 0), (1, 0), (2, 3), (3, 3), (4, 1), (5, 1), (6, 1), (7, 1)]),
        &task(&[(0, 0), (1, 0), (2, 3), (3, 3), (4, 1), (5, 1), (6, 1), (7, 1)]),
    );

    // Stage 2: the world reveals that {2,3} actually share a class with the
    // already-known tokens 0,1 and the task drops the distinction -> merge.
    r.add_stage(
        &obs(&[(0, 0), (1, 0), (2, 0), (3, 0), (4, 1), (5, 1), (6, 1), (7, 1)]),
        &task(&[(0, 0), (1, 0), (4, 1), (5, 1), (6, 1), (7, 1)]),
    );

    // Stage 3: a token moves to a wholly different class -> the old concept
    // loses the token (remove / invalidate) and a fresh concept absorbs it.
    r.add_stage(
        &obs(&[(0, 0), (1, 0), (2, 9), (3, 0), (4, 1), (5, 1), (6, 1), (7, 1)]),
        &task(&[(0, 0), (1, 0), (2, 9), (3, 0), (4, 1), (5, 1), (6, 1), (7, 1)]),
    );

    // Stage 4: previously-unseen probes enter the world -> add / retain.
    r.add_stage(
        &obs(&[
            (0, 0),
            (1, 0),
            (2, 9),
            (3, 0),
            (4, 1),
            (5, 1),
            (6, 1),
            (7, 1),
            (8, 0),
            (9, 0),
            (10, 1),
            (11, 1),
        ]),
        &task(&[
            (0, 0),
            (1, 0),
            (2, 9),
            (3, 0),
            (4, 1),
            (5, 1),
            (6, 1),
            (7, 1),
            (8, 0),
            (9, 0),
            (10, 1),
            (11, 1),
        ]),
    );

    println!("ontogenesis: non-monotonic ontology repair (U5+)");
    for stage in &r.stages {
        let ops: Vec<String> = stage
            .transitions
            .iter()
            .map(|t| format!("{}({})", t.op.name(), t.from_id))
            .collect();
        println!(
            "t{} concepts={} transitions=[{}] affected={} preserved={} structural_replaced={} \
             replayed={} acc_replayed={} total_cost={} (desc={} reason={} err={} migr={} revpen={})",
            stage.index,
            stage.ontology.concepts.len(),
            if ops.is_empty() {
                "new".to_string()
            } else {
                ops.join(" ")
            },
            stage.affected_concepts.len(),
            stage.preserved_concepts.len(),
            stage.structural_replaced,
            stage.replayed,
            stage.accumulated_replayed,
            stage.cost.total,
            stage.cost.description,
            stage.cost.reasoning,
            stage.cost.predictive_error,
            stage.cost.migration,
            stage.cost.revision_penalty,
        );
    }
    println!("{}", ontology_repair::machine_record(&r.stages));
}
