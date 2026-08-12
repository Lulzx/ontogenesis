fn main() {
    let report = supsearch::weil_scale_search::sh19a_experiment();
    println!("{}", supsearch::weil_scale_search::machine_record(&report));
}
