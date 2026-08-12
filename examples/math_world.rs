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
}
