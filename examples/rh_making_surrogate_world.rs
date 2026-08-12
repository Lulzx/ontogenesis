use supsearch::rh_making_surrogate_world::{m29b_experiment, machine_record};

fn main() {
    let report = m29b_experiment();
    println!("ontogenesis: mathematical ontogenesis (M29b surrogate)");
    println!("selected object: {}", report.selected_object);
    println!("M29 reached: {}", report.m29_reached);
    println!("{}", machine_record(&report));
}
