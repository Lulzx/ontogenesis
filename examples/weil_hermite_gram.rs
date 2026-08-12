fn main() {
    let report = supsearch::weil_hermite_gram::m29e_experiment();
    println!("{}", supsearch::weil_hermite_gram::machine_record(&report));
}
