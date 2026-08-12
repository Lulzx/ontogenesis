use supsearch::spectral_witness_search::{machine_record, sh3_experiment};

fn main() {
    let report = sh3_experiment();
    println!("ontogenesis: SH3b generic spectral witness construction");
    println!("outcome: {}", report.outcome);
    println!("M29 reached: {}", report.m29_reached);
    println!("{}", machine_record(&report));
}
