fn main() {
    let report = supsearch::weil_reduction_audit::m29g_experiment();
    println!("{}", supsearch::weil_reduction_audit::machine_record(&report));
}
