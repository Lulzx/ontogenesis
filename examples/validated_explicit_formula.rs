use supsearch::validated_explicit_formula::{machine_record, sh15_experiment};

fn main() {
    let report = sh15_experiment();
    println!("ontogenesis: SH15 validated explicit-formula evaluation");
    println!("failure: {:?}", report.failure);
    println!("{}", machine_record(&report));
}
