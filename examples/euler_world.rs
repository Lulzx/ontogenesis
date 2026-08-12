use supsearch::euler_world::{m17_experiment, machine_record, render_local};

fn main() {
    println!("ontogenesis: mathematical ontogenesis (M17)");
    println!("world: finite Euler product from multiplication behavior");
    let report = m17_experiment();
    println!(
        "local factor: {} (size {})",
        render_local(&report.discovery.local_factor),
        report.discovery.local_factor_size
    );
    for transfer in &report.transfers {
        println!(
            "{}: {} elements, {} irreducibles, ops {} -> {}, accepted {}",
            transfer.task,
            transfer.universe_size,
            transfer.irreducible_count,
            transfer.baseline_ops,
            transfer.acquired_ops,
            transfer.accepted
        );
    }
    println!("{}", machine_record(&report));
}
