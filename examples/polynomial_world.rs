use supsearch::polynomial_world::{check_factor_certificate, m13_experiment, machine_record};

fn main() {
    println!("ontogenesis: mathematical ontogenesis (M13b)");
    println!("world: ordered root observations, permutation interventions, invented invariants");
    let report = m13_experiment();
    for invariant in &report.invented_invariants {
        println!("invented invariant: {}", invariant.program.render());
    }
    for law in &report.retained_laws {
        println!(
            "discovered relation: {} => {:?}",
            law.proposed_zero.render(&report.invented_invariants),
            check_factor_certificate(law, &report.invented_invariants)
        );
    }
    println!("{}", machine_record(&report));
}
