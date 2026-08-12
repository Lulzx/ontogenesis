use supsearch::locus_world::{locus_render, m21_experiment, machine_record};

fn main() {
    println!("ontogenesis: mathematical ontogenesis (M21)");
    println!("world: toy critical symmetry locus from zero positions");
    let report = m21_experiment();
    println!("locus: {}", locus_render(&report.locus));
    for transfer in &report.transfers {
        println!(
            "{}: {} irreducibles, evals {} -> {}, exact {}",
            transfer.task,
            transfer.irreducible_count,
            transfer.baseline_evaluations,
            transfer.acquired_evaluations,
            transfer.exact
        );
    }
    println!("{}", machine_record(&report));
}
