use supsearch::real_rh_ontogenesis::{m30a_experiment, machine_record};

fn main() {
    let report = m30a_experiment();
    println!("ontogenesis: real Riemann Hypothesis (M30a)");
    println!("outcome: {}", report.outcome);
    println!("best frontier: {:?}", report.acquired.best_frontier);
    println!(
        "open assumptions: {:?}",
        report.acquired.best_frontier_check.open_assumptions
    );
    println!("{}", machine_record(&report));
}
