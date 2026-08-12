use supsearch::positive_kernel_search::{machine_record, sh14_experiment};
fn main() {
    let report = sh14_experiment();
    println!("ontogenesis: SH14 exact positive-kernel search");
    println!("weil failure: {:?}", report.weil_failure);
    println!("{}", machine_record(&report));
}
