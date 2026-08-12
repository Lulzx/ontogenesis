use supsearch::irrational_world::{
    check_irrationality_certificate, m12_experiment, machine_record,
};

fn main() {
    println!("ontogenesis: mathematical ontogenesis (M12)");
    println!(
        "world: integer-ratio witnesses, prime valuations, checked contradiction certificates"
    );
    let report = m12_experiment();
    println!("radicand: {}", report.radicand);
    println!(
        "discovered representation: {}",
        report.representation.render()
    );
    println!(
        "checked theorem: {:?}",
        check_irrationality_certificate(&report.certificate)
    );
    println!("{}", machine_record(&report));
}
