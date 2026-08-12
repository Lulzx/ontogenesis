use supsearch::functional_world::{m19_experiment, machine_record, render_factor};

fn main() {
    println!("ontogenesis: mathematical ontogenesis (M19)");
    println!("world: toy functional equation from completed-object values");
    let report = m19_experiment();
    println!(
        "transformation: s -> {}-{}s, factor: {}",
        report.discovery.transformation.a,
        report.discovery.transformation.b,
        render_factor(&report.discovery.factor)
    );
    for transfer in &report.transfers {
        println!(
            "{}: {} irreducibles, ops {} -> {}, exact {}",
            transfer.task,
            transfer.irreducible_count,
            transfer.baseline_ops,
            transfer.acquired_ops,
            transfer.exact
        );
    }
    println!("{}", machine_record(&report));
}
