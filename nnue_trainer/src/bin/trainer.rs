use bullet_lib::{
    trainer::{
        schedule::{TrainingSchedule, TrainingSteps, lr, wdl},
        settings::LocalSettings,
    },
    value::loader,
};
use nnue_trainer::{SCALE, build_trainer};

/// S2 (6.57B) + linrock_relabel_2 Oct–Dec 2023 (5.89B, no positions in check, captures, promotions or
/// unscored), shuffled together with `bullet-utils interleave`
const DATA_PATHS: &[&str] = &["data/mix/s2-relabel2q4.bin"];

const SUPERBATCHES: usize = 260;

fn main() {
    let mut trainer = build_trainer();

    // Without arguments, train from scratch. `cargo trainer -- --resume
    // checkpoints/<net_id>-<N>` continues an interrupted run at superbatch N + 1
    let start_superbatch = match resume_checkpoint() {
        Some(path) => {
            let n = path
                .trim_end_matches('/')
                .rsplit('-')
                .next()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or_else(|| panic!("checkpoint `{path}` must end in -<superbatch>"));
            trainer.load_from_checkpoint(&path);
            n + 1
        }
        None => 1,
    };

    let schedule = TrainingSchedule {
        net_id: "lucky-v7".to_string(),
        eval_scale: SCALE as f32,
        steps: TrainingSteps {
            batch_size: 16_384,
            batches_per_superbatch: 6104,
            start_superbatch,
            end_superbatch: SUPERBATCHES,
        },
        wdl_scheduler: wdl::LinearWDL {
            start: 0.0,
            end: 0.25,
        },
        lr_scheduler: lr::CosineDecayLR {
            initial_lr: 0.001,
            final_lr: 0.001 * 0.3f32.powi(5),
            final_superbatch: SUPERBATCHES,
        },
        save_rate: 30,
    };

    let settings = LocalSettings {
        threads: 4,
        test_set: None,
        output_directory: "checkpoints",
        batch_queue_size: 64,
    };

    let data_loader = loader::DirectSequentialDataLoader::new(DATA_PATHS);

    trainer.run(&schedule, &settings, &data_loader);
}

fn resume_checkpoint() -> Option<String> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None => None,
        Some("--resume") => Some(args.next().expect("--resume needs a checkpoint path")),
        Some(other) => {
            panic!("unknown argument `{other}` (usage: trainer [--resume <checkpoint>])")
        }
    }
}
