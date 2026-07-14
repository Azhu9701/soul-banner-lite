//! Soul Agent SDK — Conference Mode Example
//!
//! Demonstrates the multi-agent conference orchestration pattern.
//!
//! Usage: cargo run -p soul-agent --example conference_mode

use foundation::SoulAgentConfig;

fn main() {
    println!("Soul Agent SDK — Conference Mode");
    println!();

    let _config = SoulAgentConfig::from_data_dir("./data");

    // Conference mode runs multiple souls in parallel with cross-detection:
    //
    // 1. Each soul generates its response independently
    // 2. CrossDetector monitors for contradictions between souls
    // 3. Synthesis officer produces a unified analysis
    // 4. TopologyPlanner selects optimal orchestration strategy
    //
    println!("TopologyPlanner decision tree:");
    println!("  complexity < 0.3 + budget → Minimal (1 soul)");
    println!("  diversity > 0.7 + complex  → FullMesh + cross_detect");
    println!("  2 souls                     → Oppositional (debate)");
    println!("  default                     → ClusteredParallel");
    println!();

    println!("CrossDetector collision types:");
    println!("  Contradiction          — 但是、然而、不同意");
    println!("  PerspectiveDifference  — 从另一个角度、换个视角");
    println!("  PremiseDisagreement    — 假设分歧");
    println!("  SupplementaryChallenge — 补充质疑");
    println!();

    println!("Session flow:");
    println!("  input = PossessionInput {{");
    println!("      task: \"四天工作制可行性分析\",");
    println!("      souls: vec![\"经济学家\", \"HR总监\", \"工会代表\"],");
    println!("      mode: \"conference\",");
    println!("  }}");
    println!();
    println!("  engine.start_possession(input, tx).await?;");
    println!("  // Stream: SoulStarted → SoulChunk* → SoulDone →");
    println!("  //         SynthesisChunk* → Collision? → SessionComplete");
}
