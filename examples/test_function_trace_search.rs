use supsearch::test_function_trace_search::{machine_record, sh8_experiment};

fn main() {
    let report = sh8_experiment();
    println!("ontogenesis: SH8 even test-function trace search");
    println!("normalization: {}", report.common_normalization);
    println!("{}", machine_record(&report));
}
