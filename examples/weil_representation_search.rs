fn main() {
    let report = supsearch::weil_representation_search::sh19_experiment();
    println!(
        "{}",
        supsearch::weil_representation_search::machine_record(&report)
    );
}
