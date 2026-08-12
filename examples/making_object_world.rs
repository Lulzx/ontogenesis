use supsearch::making_object_world::{m25_experiment, machine_record};

fn main() {
    println!("ontogenesis: mathematical ontogenesis (M25)");
    println!("world: toy RH-making object with checked forcing theorem");
    let report = m25_experiment();
    println!(
        "family: {}, forcing: {}, non-vacuous: {}",
        report.family_size, report.forcing_implication, report.non_vacuous
    );
    for transfer in &report.transfers {
        println!(
            "{}: evals {} -> {}, forcing {}, provenance {}, exact {}",
            transfer.task,
            transfer.baseline_evaluations,
            transfer.acquired_evaluations,
            transfer.forcing,
            transfer.provenance,
            transfer.exact
        );
    }
    println!("{}", machine_record(&report));
}
