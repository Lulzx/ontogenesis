use supsearch::spectral_counting_obstruction::{machine_record, sh9_experiment};

fn main() {
    let report = sh9_experiment();
    println!("ontogenesis: SH9 spectral-counting obstruction");
    println!(
        "selected family eliminated: {}",
        report.selected_family_eliminated
    );
    println!("{}", machine_record(&report));
}
