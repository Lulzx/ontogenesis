use supsearch::rh_equivalence_world::{m28_experiment, machine_record};

fn main() {
    let report = m28_experiment();
    println!("ontogenesis: mathematical ontogenesis (M28)");
    println!("selected equivalence: {}", report.selected_predicate);
    println!("RH proved: {}", report.rh_proved);
    println!("{}", machine_record(&report));
}
