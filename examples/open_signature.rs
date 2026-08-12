use supsearch::open_signature;

fn main() {
    let report = open_signature::run_experiment();
    println!("U5 open-world recursive-signature ontogenesis");
    for (index, stage) in report.stages.iter().enumerate() {
        let incumbent = stage.incumbent.as_ref().unwrap();
        println!(
            "t{} observed={:?} preferred={:?} compatible={} aliases={} revision={:?} provisional={} replayed={}",
            index + 1,
            stage.observed,
            incumbent.profile,
            stage.accounting.compatible_classes,
            incumbent.aliases.len(),
            stage.revision,
            stage.provisional,
            stage.replayed,
        );
    }
    for (index, economics) in report.economics.iter().enumerate() {
        println!(
            "t{} proposals learned={} uniform={} oracle={} supplied-complete={} irrelevant={} misleading={} net={}",
            index + 1,
            economics.learned.proposals,
            economics.uniform.proposals,
            economics.oracle.proposals,
            economics.supplied_complete.proposals,
            economics.irrelevant.proposals,
            economics.misleading.proposals,
            economics.net_gain,
        );
    }
    println!(
        "unary executable structure validated={}",
        report.unary_structure_validated
    );
    println!("{}", open_signature::machine_record(&report));
}
