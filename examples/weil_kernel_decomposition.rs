fn main() {
    let report = supsearch::weil_kernel_decomposition::m29h_experiment();
    println!(
        "{}",
        supsearch::weil_kernel_decomposition::machine_record(&report)
    );
}
