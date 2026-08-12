use supsearch::validated_archimedean::{machine_record, sh16_experiment};
fn main() {
    let report = sh16_experiment();
    println!("ontogenesis: SH16 validated archimedean quadrature");
    println!("failure: {:?}", report.failure);
    println!("{}", machine_record(&report));
}
