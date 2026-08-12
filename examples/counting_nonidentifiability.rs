use supsearch::counting_nonidentifiability::{machine_record, sh12_experiment};

fn main() {
    let report = sh12_experiment();
    println!("ontogenesis: SH12 counting non-identifiability");
    println!(
        "counting route eliminated: {}",
        report.counting_route_eliminated
    );
    println!("{}", machine_record(&report));
}
