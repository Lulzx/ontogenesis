use supsearch::conjecture_world::{conjecture_render, m23_experiment, machine_record};

fn main() {
    println!("ontogenesis: mathematical ontogenesis (M23)");
    println!("world: toy RH-like conjecture from partial zero evidence");
    let report = m23_experiment();
    println!("conjecture: {}", conjecture_render(&report.conjecture));
    for transfer in &report.transfers {
        println!(
            "{}: {} irreducibles, evals {} -> {}, valid {}, falsifier {:?}",
            transfer.task,
            transfer.irreducible_count,
            transfer.baseline_evaluations,
            transfer.conjectured_evaluations,
            transfer.held_out_zeros_valid,
            transfer.falsifier
        );
    }
    println!("{}", machine_record(&report));
}
