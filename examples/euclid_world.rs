use supsearch::euclid_world::{check_escape_certificate, m11_experiment, machine_record};

fn main() {
    println!("ontogenesis: mathematical ontogenesis (M11)");
    println!("world: finite collections, generic folds/arithmetic, existential prime-divisor certificate");
    let report = m11_experiment();
    println!("conjecture: {}", report.conjecture);
    println!(
        "discovered auxiliary object: {}",
        report.construction.render()
    );
    println!(
        "checked theorem: {:?}",
        check_escape_certificate(&report.certificate)
    );
    println!("{}", machine_record(&report));
}
