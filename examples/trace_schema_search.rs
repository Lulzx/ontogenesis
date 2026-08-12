use supsearch::trace_schema_search::{machine_record, sh6b_experiment};

fn main() {
    let report = sh6b_experiment();
    println!("ontogenesis: SH6b cross-domain trace-schema calibration");
    println!("retained schema: {}", report.retained_schema);
    println!("passed: {}", report.sh6b_passed);
    println!("{}", machine_record(&report));
}
