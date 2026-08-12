use supsearch::completion_world::{completion_render, m20_experiment, machine_record};

fn main() {
    println!("ontogenesis: mathematical ontogenesis (M20)");
    println!("world: toy completed object with simple symmetry");
    let report = m20_experiment();
    println!("completion: {}", completion_render(&report.completion));
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
