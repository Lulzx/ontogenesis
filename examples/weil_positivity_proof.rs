fn main() {
    let report = supsearch::weil_positivity_proof::positivity_proof();
    println!(
        "{}",
        supsearch::weil_positivity_proof::machine_record(&report)
    );
}
