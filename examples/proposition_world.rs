use supsearch::proposition_world::{m10_experiment, machine_record};

fn main() {
    println!("ontogenesis: mathematical ontogenesis (M10)");
    println!(
        "world: typed integer polynomials, divisibility propositions, checked proof certificates"
    );
    let report = m10_experiment();
    println!("original: forall n, {}", report.original.render());
    println!(
        "discovered alternative: forall n, {}",
        report.alternative.render()
    );
    println!(
        "proofs: forward_checked=true backward_checked=true original_cost={} alternative_cost={}",
        report.original_proof.reasoning_cost, report.alternative_proof.reasoning_cost
    );
    println!("transfer to modulus 3 / cube: {}", report.transfer_verified);
    println!("{}", machine_record(&report));
}
