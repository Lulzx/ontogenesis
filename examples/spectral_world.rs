use supsearch::spectral_world::{m16_experiment, machine_record, render_predicate, Route};

fn main() {
    println!("ontogenesis: mathematical ontogenesis (M16)");
    println!("world: toy spectral regularity from transition observations");
    let report = m16_experiment();
    println!(
        "predicate: {} (size {})",
        render_predicate(&report.discovery.predicate),
        report.discovery.predicate_size
    );
    println!(
        "separated training exactly: {}",
        report.discovery.separated_exactly
    );
    for transfer in &report.transfers {
        println!(
            "{} {:?}: {} -> {} checks, exact {}, decomposition {}, long-horizon {} -> {}",
            if transfer.route == Route::Admitted {
                "Admitted"
            } else {
                "Declined"
            },
            transfer.task,
            transfer.baseline_checks,
            transfer.acquired_checks,
            transfer.exact_winner,
            transfer.decomposition_checked,
            transfer.long_horizon_baseline_ops,
            transfer.long_horizon_acquired_ops
        );
    }
    println!("{}", machine_record(&report));
}
