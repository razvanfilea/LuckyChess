use bullet_lib::{
    game::{inputs::ChessBucketsMirrored, outputs::MaterialCount},
    nn::{
        InitSettings, Shape,
        optimiser::{AdamW, AdamWOptimiser},
    },
    trainer::save::SavedFormat,
    value::{ValueTrainer, ValueTrainerBuilder},
};

include!("../../chess_engine/src/nnue/constants.rs");

/// Feature weights are stored as i8 at QA = 127: 0.99 * 127 rounds to 126
const L0_CLIP: f32 = 0.99;

pub fn build_trainer()
-> ValueTrainer<AdamWOptimiser, ChessBucketsMirrored, MaterialCount<OUTPUT_BUCKETS>> {
    ValueTrainerBuilder::default()
        .dual_perspective()
        .optimiser(AdamW)
        .inputs(ChessBucketsMirrored::new(BUCKET_LAYOUT.map(usize::from)))
        .output_buckets(MaterialCount::<OUTPUT_BUCKETS>)
        .save_format(&[
            SavedFormat::id("l0w")
                .transform(|store, weights| {
                    let factorizer = store.get("l0f").values.f32().repeat(INPUT_BUCKETS);
                    weights
                        .into_iter()
                        .zip(factorizer)
                        .map(|(a, b)| (a + b).clamp(-L0_CLIP, L0_CLIP))
                        .collect()
                })
                .round()
                .quantise::<i8>(QA as i16),
            SavedFormat::id("l0b").round().quantise::<i16>(QA as i16),
            SavedFormat::id("l1w")
                .round()
                .transpose()
                .quantise::<i16>(QB as i16),
            SavedFormat::id("l1b")
                .round()
                .quantise::<i16>((QA * QB) as i16),
        ])
        .loss_fn(|output, target| output.sigmoid().squared_error(target))
        .build(|builder, stm_inputs, ntm_inputs, buckets| {
            let l0f =
                builder.new_weights("l0f", Shape::new(HIDDEN_SIZE, 768), InitSettings::Zeroed);
            let expanded_factorizer = l0f.repeat(INPUT_BUCKETS);

            let mut l0 = builder.new_affine("l0", 768 * INPUT_BUCKETS, HIDDEN_SIZE);
            // Clip the merged weights so the forward pass sees what the i8 export stores
            l0.weights = (l0.weights + expanded_factorizer).clip_pass_through_grad(-L0_CLIP, L0_CLIP);

            let l1 = builder.new_affine("l1", 2 * HIDDEN_SIZE, OUTPUT_BUCKETS);

            let stm_hidden = l0.forward(stm_inputs).screlu();
            let ntm_hidden = l0.forward(ntm_inputs).screlu();
            let hidden_layer = stm_hidden.concat(ntm_hidden);
            l1.forward(hidden_layer).select(buckets)
        })
}
