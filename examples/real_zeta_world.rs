use supsearch::real_zeta_world::{m26_experiment, machine_record};

fn main() {
    let report = m26_experiment();
    println!("ontogenesis: mathematical ontogenesis (M26)");
    println!("selected: {}", report.selected_formula);
    println!("cold candidates: {}", report.cold.candidate_evaluations);
    println!(
        "transferred candidates: {}",
        report.transferred.candidate_evaluations
    );
    println!("{}", machine_record(&report));
}
