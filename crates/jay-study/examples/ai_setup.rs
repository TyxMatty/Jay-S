//! `cargo run --release --example ai_setup -- <data folder> [standard|light]`: downloads Jay-AI's
//! runtime and a model the way the app does (pinned SHA-256, resumable), for tests on a dev PC.
use jay_study::ai::{AiPaths, ModelChoice};
use std::sync::atomic::AtomicBool;

fn main() {
    let mut args = std::env::args().skip(1);
    let root = std::path::PathBuf::from(args.next().expect("data folder"));
    let choice = if args.next().as_deref() == Some("light") { ModelChoice::Light } else { ModelChoice::Standard };
    let paths = AiPaths::new(&root);
    let cancel = AtomicBool::new(false);
    let started = std::time::Instant::now();
    let report = |what: &'static str| {
        let mut last = 0u64;
        move |done: u64, total: u64| {
            if done * 20 / total.max(1) != last {
                last = done * 20 / total.max(1);
                println!("{what}: {} / {} MB", done >> 20, total >> 20);
            }
        }
    };
    if !paths.runtime_ready() {
        paths.install_runtime(&mut report("runtime"), &cancel).expect("runtime");
    }
    if !paths.model_ready(choice) {
        paths.install_model(choice, &mut report("model"), &cancel).expect("model");
    }
    println!("ready in {:?}: {}", started.elapsed(), paths.model_path(choice).display());
}
