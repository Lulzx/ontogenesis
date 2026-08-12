use supsearch::positive_functional_operator::{machine_record, sh13_experiment};

fn main() {
    let report = sh13_experiment();
    println!("ontogenesis: SH13 positive-functional operator constructor");
    println!("missing: {}", report.first_missing_premise);
    println!("{}", machine_record(&report));
}
