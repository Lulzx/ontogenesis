fn main() {
    let report = supsearch::weil_entry_assembly::sh18b_experiment();
    println!(
        "{}",
        supsearch::weil_entry_assembly::machine_record(&report)
    );
}
