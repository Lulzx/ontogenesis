fn main() {
    let report = supsearch::weil_forcing_object::m29c_experiment();
    println!(
        "{}",
        supsearch::weil_forcing_object::machine_record(&report)
    );
}
