// Ontogenesis experiment: causal ontology from intervention responses
// (Direction F, §9).
//
// Three binary variables A, B, C form a hidden directed acyclic graph with a
// deterministic boolean function per node. Passive observation yields the joint
// set of natural outcomes, which leaves a whole Markov-equivalence class of
// causal models indistinguishable (e.g. the chain A->B->C and the fork
// A<-B->C are passively identical). The learner must instead *intervene* —
// force a variable to a value and read the downstream response — to separate
// correlation from mechanism from intervention response, without the graph
// names being supplied.
use supsearch::causal_ontology::{
    self, chain_model, fork_model, enumerate_models, observe, run_causal,
};

fn main() {
    let names = ["A", "B", "C"];

    let chain = chain_model();
    let fork = fork_model();

    println!("ontogenesis: causal ontology from intervention responses (F)");
    println!(
        "model pool (acyclic, bounded functions) = {}",
        enumerate_models().len()
    );

    // Passive observation alone cannot separate chain from fork.
    println!(
        "chain passive={{ {:?} }}  fork passive={{ {:?} }}",
        observe(&chain),
        observe(&fork)
    );
    println!(
        "passively distinguishable? {} (must be false: Markov-equivalent)",
        observe(&chain) != observe(&fork)
    );

    // Run causal inference from passive data plus adaptive interventions.
    let passive = observe(&chain);
    let r = run_causal(&chain, &passive, &names);

    println!(
        "passive_candidates={} passive_distinguished={}",
        r.passive_candidates, r.passive_distinguished
    );
    for s in &r.interventions {
        println!(
            "  intervene {}={} (gain={}) -> candidates {} -> {}",
            names[s.variable as usize],
            s.value,
            s.gain,
            s.candidates_before,
            s.candidates_after
        );
    }
    println!(
        "final_candidates={} true_recovered={} causal_structure_identified={}",
        r.final_candidates, r.true_recovered, r.causal_structure_identified
    );
    println!("{}", causal_ontology::machine_record(&r, &names));

}
