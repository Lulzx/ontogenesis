use supsearch::prime_trace_search::{machine_record, sh7_experiment};

fn main() {
    let report = sh7_experiment();
    println!("ontogenesis: SH7b exact prime-Jacobi trace retry");
    println!("coefficients: {:?}", report.discovered_coefficients);
    println!("frontier: {:?}", report.frontier);
    println!("{}", machine_record(&report));
}
