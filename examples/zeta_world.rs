use supsearch::zeta_world::{m18_experiment, machine_record};

fn main() {
    println!("ontogenesis: mathematical ontogenesis (M18)");
    println!("world: compact toy zeta object from exact integer special values");
    let report = m18_experiment();
    println!(
        "local factor: {} (size {})",
        report.discovery.local_factor.render(),
        report.discovery.local_factor_size
    );
    for transfer in &report.transfers {
        println!(
            "{}: {} elements, {} irreducibles, ops {} -> {}, exact {}, pole order {}",
            transfer.task,
            transfer.universe_size,
            transfer.irreducible_count,
            transfer.baseline_ops,
            transfer.acquired_ops,
            transfer.exact,
            transfer.formal_pole_order
        );
    }
    println!("{}", machine_record(&report));
}
