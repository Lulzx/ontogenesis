use supsearch::validated_prime_power::{machine_record, sh17_experiment};
fn main() {
    let report = sh17_experiment();
    println!("ontogenesis: SH17 validated prime-power tail");
    println!("nested: {}", report.enclosures_nested);
    println!("{}", machine_record(&report));
}
