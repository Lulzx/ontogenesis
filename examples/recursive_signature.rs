use supsearch::{recursive_signature, term};

fn main() {
    let report = recursive_signature::run_experiment();
    let structure = report.discovery.structure.as_ref().unwrap();
    println!("U4 recursive-signature ontogenesis");
    println!(
        "semantic profile: {:?}; syntax aliases: {}",
        structure.signature_class.profile,
        structure.signature_class.aliases.len()
    );
    println!(
        "smallest aliases: {:?}",
        structure
            .signature_class
            .aliases
            .iter()
            .take(4)
            .map(|s| s.code())
            .collect::<Vec<_>>()
    );
    println!(
        "constructor programs: {:?}",
        structure
            .constructors
            .iter()
            .map(|program| term::show(program))
            .collect::<Vec<_>>()
    );
    println!("alpha: {}", term::show(&structure.alpha));
    println!("mediator generator: {}", term::show(&structure.generator));
    println!("action: {}", term::show(&structure.action));
    println!(
        "identifiability: weak={} rich={}; protected: commutes={} unique={} exhaustive={}",
        report.discovery.weak_identifiable,
        report.discovery.rich_identifiable,
        report.protected_commutes,
        report.protected_unique,
        report.uniqueness_exhaustive,
    );
    println!(
        "proposals: learned={} uniform={} oracle={} supplied-F={} irrelevant={} universal={}",
        report.economics.learned.proposals,
        report.economics.uniform.proposals,
        report.economics.oracle.proposals,
        report.economics.supplied_f.proposals,
        report.economics.irrelevant.proposals,
        report.economics.universal.proposals,
    );
    println!(
        "discovery charge={}; reuse horizon={}; net gain={}",
        report.discovery.charged_discovery_cost,
        report.economics.reuse_horizon,
        report.economics.net_gain,
    );
    println!("{}", recursive_signature::machine_record(&report));
}
