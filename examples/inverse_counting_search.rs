use supsearch::inverse_counting_search::{machine_record, sh11_experiment};

fn main() {
    let report = sh11_experiment();
    println!("ontogenesis: SH11 generic inverse-counting constructor");
    println!("retained: {}", report.retained_schema);
    println!("{}", machine_record(&report));
}
