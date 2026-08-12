use supsearch::hidden_zeros_world::{m22_experiment, machine_record};

fn main() {
    println!("ontogenesis: mathematical ontogenesis (M22)");
    println!("world: hidden toy zeros from exact oscillation signals");
    let report = m22_experiment();
    println!(
        "oscillators: {}",
        report
            .model
            .iter()
            .map(|(u, weight)| format!("u={u}:w={weight}:v={}", 1 - u))
            .collect::<Vec<_>>()
            .join(";")
    );
    for transfer in &report.transfers {
        println!(
            "{}: {} irreducibles, evals {} -> {}, signal {}, locations {}",
            transfer.task,
            transfer.irreducible_count,
            transfer.baseline_evaluations,
            transfer.acquired_evaluations,
            transfer.exact_signal,
            transfer.exact_locations
        );
    }
    println!("{}", machine_record(&report));
}
