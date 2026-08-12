use supsearch::equivalence_world::{m24_experiment, machine_record, predicate_render};

fn main() {
    println!("ontogenesis: mathematical ontogenesis (M24)");
    println!("world: toy-RH equivalence with checked bidirectional certificates");
    let report = m24_experiment();
    println!("Q: {}", predicate_render(report.predicate));
    for transfer in &report.transfers {
        println!(
            "{}: forward {}, backward {}, ops {} -> {} + proof {}",
            transfer.task,
            transfer.forward,
            transfer.backward,
            transfer.baseline_ops,
            transfer.q_ops,
            transfer.proof_comparisons
        );
    }
    println!("{}", machine_record(&report));
}
