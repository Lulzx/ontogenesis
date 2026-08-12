// Ontogenesis experiment: mathematical ontogenesis (Direction M1).
//
// Mathematics is treated as a world W = (S, A, T, O). Given Pythagorean-triple
// observations (x, y) -> d, the agent must invent an expression that explains
// them and generalizes to unseen points, then reuse it as a concept to make
// later reasoning cheaper. The concept of Euclidean distance is never supplied.
use supsearch::math_world::{
    compression_report, discover_concept, machine_record, transfer_report,
};

fn main() {
    // Training observations: Pythagorean triples.
    let training = vec![
        (3.0, 4.0, 5.0),
        (5.0, 12.0, 13.0),
        (8.0, 15.0, 17.0),
        (7.0, 24.0, 25.0),
    ];
    // Held-out observations: unseen Pythagorean triples.
    let held_out = vec![
        (20.0, 21.0, 29.0),
        (9.0, 40.0, 41.0),
        (12.0, 35.0, 37.0),
        (28.0, 45.0, 53.0),
    ];

    println!("ontogenesis: mathematical ontogenesis (M1)");
    println!("world: arithmetic expressions over x,y with + - * / sqrt");
    println!("training observations={} held-out={}", training.len(), held_out.len());

    // 1. Invent the distance concept.
    let concept = discover_concept(&training, &held_out, 8).expect("must discover");
    println!(
        "discovered expression: {} (size {}, discovery_cost {})",
        concept.expr.to_string(),
        concept.expr.size(),
        concept.discovery_cost
    );
    println!("generalizes to held-out: {}", concept.generalizes);

    // 2. Transfer: predicting held-out distances with vs. without the concept.
    let tr = transfer_report(&concept, &held_out);
    println!(
        "transfer: concept_reasoning_cost={} baseline_reasoning_cost={} saving={}",
        tr.concept_reasoning_cost, tr.baseline_reasoning_cost, tr.transfer_saving
    );

    // 3. Compression: the concept shortens the description of the observations.
    let comp = compression_report(&concept, &training, &held_out);
    println!(
        "compression: raw_observations={} raw_tokens={} concept_tokens={} gain={}",
        comp.raw_observations, comp.raw_tokens, comp.concept_tokens, comp.compression_gain
    );

    println!("{}", machine_record(&concept, &tr, &comp));
    m2();
}

// ---------------------------------------------------------------------------
// Direction M2: invent the circle invariant.
// ---------------------------------------------------------------------------
use supsearch::math_world::{
    discover_invariant, invariant_compression, invariant_transfer, machine_record_m2,
};

fn m2() {
    // Members: points on a circle of radius 5 (hidden class).
    let members = vec![(3.0, 4.0), (4.0, 3.0), (-3.0, 4.0), (0.0, 5.0)];
    // Non-members: points not on the circle.
    let non_members = vec![(1.0, 1.0), (2.0, 2.0), (5.0, 5.0), (1.0, 3.0)];
    // Held-out members / non-members for generalization.
    let held_members = vec![(0.0, -5.0), (-4.0, -3.0), (3.0, -4.0), (-5.0, 0.0)];
    let held_non_members = vec![(6.0, 1.0), (2.0, 7.0), (4.0, 4.0), (7.0, 2.0)];

    println!();
    println!("ontogenesis: mathematical ontogenesis (M2)");
    println!(
        "world: points on a hidden circle; members={} non-members={} held-members={} held-non-members={}",
        members.len(),
        non_members.len(),
        held_members.len(),
        held_non_members.len()
    );

    // 1. Invent the circle invariant.
    let inv = discover_invariant(&members, &non_members, &held_members, &held_non_members, 7)
        .expect("must discover");
    println!(
        "discovered invariant: {} = {:.0} (size {}, discovery_cost {})",
        inv.expr.to_string(),
        inv.constant,
        inv.expr.size(),
        inv.discovery_cost
    );
    println!("generalizes to held-out: {}", inv.generalizes);

    // 2. Transfer: classifying held-out points with vs. without the invariant.
    let held = held_members.len() + held_non_members.len();
    let tr = invariant_transfer(&inv, held);
    println!(
        "transfer: concept_reasoning_cost={} baseline_reasoning_cost={} saving={}",
        tr.concept_reasoning_cost, tr.baseline_reasoning_cost, tr.transfer_saving
    );

    // 3. Compression: the invariant compresses the class.
    let comp = invariant_compression(&inv, &members, &held_members);
    println!(
        "compression: raw_points={} raw_tokens={} concept_tokens={} gain={}",
        comp.raw_points, comp.raw_tokens, comp.concept_tokens, comp.compression_gain
    );

    println!(
        "{}",
        machine_record_m2(
            &inv,
            &tr,
            &comp,
            members.len(),
            non_members.len(),
            held_members.len(),
            held_non_members.len()
        )
    );
}
