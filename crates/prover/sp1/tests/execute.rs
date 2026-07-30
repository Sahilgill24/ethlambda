//! Integration test for the SP1 STF prover's `execute` path.
//!
//! `execute` runs the guest in the SP1 RISC-V executor without proving, so it
//! is cheap enough for regular runs while still exercising the real guest
//! and the `StfInput`/`StfPublicValues` serialization/deserialization. 


use ethlambda_prover_core::{StfInput, StfProver, StfPublicValues};
use ethlambda_prover_sp1::Sp1Prover;
use ethlambda_state_transition::{process_block, process_slots};
use ethlambda_types::{
    block::{Block, BlockBody},
    primitives::{H256, HashTreeRoot as _},
    state::{State, Validator},
};

const GENESIS_TIME: u64 = 0;
const NUM_VALIDATORS: u64 = 4;


fn valid_transition() -> (StfInput, StfPublicValues) {
    let validators = (0..NUM_VALIDATORS)
        .map(|index| Validator {
            attestation_pubkey: [0u8; 52],
            proposal_pubkey: [0u8; 52],
            index,
        })
        .collect();
    let pre_state = State::from_genesis(GENESIS_TIME, validators);

    // Parent root as `process_slots` will leave it: the genesis header with its
    // `state_root` filled in (mirrors block_builder.rs).
    let mut parent_header = pre_state.latest_block_header.clone();
    parent_header.state_root = pre_state.hash_tree_root();
    let parent_root = parent_header.hash_tree_root();

    let mut block = Block {
        slot: 1,
        proposer_index: 1 % NUM_VALIDATORS, 
        parent_root,
        state_root: H256::ZERO,
        body: BlockBody::default(),
    };

    let mut scratch = pre_state.clone();
    process_slots(&mut scratch, block.slot).expect("process_slots");
    process_block(&mut scratch, &block).expect("process_block");
    block.state_root = scratch.hash_tree_root();

    let expected = StfPublicValues {
        pre_state_root: pre_state.hash_tree_root(),
        block_root: block.hash_tree_root(),
        post_state_root: block.state_root,
    };
    (StfInput::new(pre_state, block), expected)
}

#[tokio::test]
async fn execute_commits_expected_roots() {
    // generate the inputs and the output expected state. 
    let (input, expected) = valid_transition();

    let prover = Sp1Prover::new().await;
    let execution_result = prover.execute(&input).await.expect("execute failed");

    assert_eq!(execution_result.pre_state_root, expected.pre_state_root, "pre_state_root");
    assert_eq!(execution_result.block_root, expected.block_root, "block_root");
    assert_eq!(
        execution_result.post_state_root, expected.post_state_root,
        "post_state_root"
    );
}
