fn main() {
    let report = supsearch::weil_positive_kernel::m29f_experiment();
    println!("{}", supsearch::weil_positive_kernel::machine_record(&report));
}
