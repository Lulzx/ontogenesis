use supsearch::critical_line_world::{m27_experiment, machine_record};

fn main() {
    let report = m27_experiment();
    println!("ontogenesis: mathematical ontogenesis (M27)");
    println!("selected conjecture: {}", report.selected_locus);
    println!("proof: {}", report.proof);
    println!("{}", machine_record(&report));
}
