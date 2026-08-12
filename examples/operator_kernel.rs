use supsearch::operator_kernel::{machine_record, sh4_experiment};

fn main() {
    let report = sh4_experiment();
    println!("ontogenesis: SH4c typed infinite-operator proof kernel");
    println!("kernel passed: {}", report.sh4_passed);
    println!("M29 reached: {}", report.m29_reached);
    println!("{}", machine_record(&report));
}
