use super::*;

const EMPTY_HASH: &str = "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419\
                          d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce";
const ABC_HASH: &str = "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d1\
                        7d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923";
const ONE_BLOCK_HASH: &str = "865939e120e6805438478841afb739ae4250cf372653078a065cdcfffca4caf7\
                              98e6d462b65d658fc165782640eded70963449ae1500fb0f24981d7727e22c41";
const THOUSAND_HASH: &str = "1ee4e51ecab5210a518f26150e882627ec839967f19d763e1508b12cfefed148\
                             58f6a1c9d1f969bc224dc9440f5a6955277e755b9c513f9ba4421c5e50c8d787";

#[test]
fn test_vectors() {
    let io = &[
        (&b""[..], EMPTY_HASH),
        (&b"abc"[..], ABC_HASH),
        (&[0; BLOCKBYTES], ONE_BLOCK_HASH),
        (&[0; 1000], THOUSAND_HASH),
    ];
    // Test each input all at once.
    for &(input, output) in io {
        let hash = blake2b(input);
        assert_eq!(&hash.to_hex(), output, "hash mismatch");
    }
    // Now in two chunks. This is especially important for the ONE_BLOCK case, because it would be
    // a mistake for update() to call compress, even though the buffer is full.
    for &(input, output) in io {
        let mut state = State::new();
        let split = input.len() / 2;
        state.update(&input[..split]);
        assert_eq!(split as u128, state.count());
        state.update(&input[split..]);
        assert_eq!(input.len() as u128, state.count());
        let hash = state.finalize();
        assert_eq!(&hash.to_hex(), output, "hash mismatch");
    }
    // Now one byte at a time.
    for &(input, output) in io {
        let mut state = State::new();
        let mut count = 0;
        for &b in input {
            state.update(&[b]);
            count += 1;
            assert_eq!(count, state.count());
        }
        let hash = state.finalize();
        assert_eq!(&hash.to_hex(), output, "hash mismatch");
    }
}

#[test]
fn test_multiple_finalizes() {
    let mut state = State::new();
    assert_eq!(&state.finalize().to_hex(), EMPTY_HASH, "hash mismatch");
    assert_eq!(&state.finalize().to_hex(), EMPTY_HASH, "hash mismatch");
    assert_eq!(&state.finalize().to_hex(), EMPTY_HASH, "hash mismatch");
    state.update(b"abc");
    assert_eq!(&state.finalize().to_hex(), ABC_HASH, "hash mismatch");
    assert_eq!(&state.finalize().to_hex(), ABC_HASH, "hash mismatch");
    assert_eq!(&state.finalize().to_hex(), ABC_HASH, "hash mismatch");
}

#[cfg(feature = "std")]
#[test]
fn test_write() {
    use std::io::prelude::*;

    let mut state = State::new();
    state.write_all(&[0; 1000]).unwrap();
    let hash = state.finalize();
    assert_eq!(&hash.to_hex(), THOUSAND_HASH, "hash mismatch");
}

// You can check this case against the equivalent Python:
//
// import hashlib
// hashlib.blake2b(
//     b'foo',
//     digest_size=18,
//     key=b"bar",
//     salt=b"bazbazbazbazbazb",
//     person=b"bing bing bing b",
//     fanout=2,
//     depth=3,
//     leaf_size=0x04050607,
//     node_offset=0x08090a0b0c0d0e0f,
//     node_depth=16,
//     inner_size=17,
//     last_node=True,
// ).hexdigest()
#[test]
fn test_all_parameters() {
    let hash = Params::new()
        .hash_length(18)
        // Make sure a shorter key properly overwrites a longer one.
        .key(b"not the real key")
        .key(b"bar")
        .salt(b"bazbazbazbazbazb")
        .personal(b"bing bing bing b")
        .fanout(2)
        .max_depth(3)
        .max_leaf_length(0x04050607)
        .node_offset(0x08090a0b0c0d0e0f)
        .node_depth(16)
        .inner_hash_length(17)
        .to_state()
        .set_last_node(true)
        .update(b"foo")
        .finalize();
    assert_eq!("ec0f59cb65f92e7fcca1280ba859a6925ded", &hash.to_hex());
}

#[test]
fn test_all_parameters_blake2bp() {
    let hash = blake2bp::Params::new()
        .hash_length(18)
        // Make sure a shorter key properly overwrites a longer one.
        .key(b"not the real key")
        .key(b"bar")
        .to_state()
        .update(b"foo")
        .finalize();
    assert_eq!("8c54e888a8a01c63da6585c058fe54ea81df", &hash.to_hex());
}

#[test]
#[should_panic]
fn test_short_hash_length_panics() {
    Params::new().hash_length(0);
}

#[test]
#[should_panic]
fn test_long_hash_length_panics() {
    Params::new().hash_length(OUTBYTES + 1);
}

#[test]
#[should_panic]
fn test_long_key_panics() {
    Params::new().key(&[0; KEYBYTES + 1]);
}

#[test]
#[should_panic]
fn test_long_salt_panics() {
    Params::new().salt(&[0; SALTBYTES + 1]);
}

#[test]
#[should_panic]
fn test_long_personal_panics() {
    Params::new().personal(&[0; PERSONALBYTES + 1]);
}

#[test]
#[should_panic]
fn test_zero_max_depth_panics() {
    Params::new().max_depth(0);
}

#[test]
#[should_panic]
fn test_long_inner_hash_length_panics() {
    Params::new().inner_hash_length(OUTBYTES + 1);
}

#[test]
#[should_panic]
fn test_blake2bp_short_hash_length_panics() {
    blake2bp::Params::new().hash_length(0);
}

#[test]
#[should_panic]
fn test_blake2bp_long_hash_length_panics() {
    blake2bp::Params::new().hash_length(OUTBYTES + 1);
}

#[test]
#[should_panic]
fn test_blake2bp_long_key_panics() {
    blake2bp::Params::new().key(&[0; KEYBYTES + 1]);
}

#[test]
fn test_update4() {
    const INPUT_PREFIX: &[u8] = b"foobarbaz";

    // Define an inner test run function, because we're going to run different permutations of
    // states and inputs.
    fn test_run(
        state0: &mut State,
        state1: &mut State,
        state2: &mut State,
        state3: &mut State,
        input0: &[u8],
        input1: &[u8],
        input2: &[u8],
        input3: &[u8],
    ) {
        // Compute the expected hashes the normal way, using cloned copies.
        let expected0 = state0.clone().update(input0).finalize();
        let expected1 = state1.clone().update(input1).finalize();
        let expected2 = state2.clone().update(input2).finalize();
        let expected3 = state3.clone().update(input3).finalize();

        // Now do the same thing using the parallel interface.
        update4(
            state0, state1, state2, state3, input0, input1, input2, input3,
        );
        let output = finalize4(state0, state1, state2, state3);

        assert_eq!(expected0, output[0]);
        assert_eq!(expected1, output[1]);
        assert_eq!(expected2, output[2]);
        assert_eq!(expected3, output[3]);
    }

    // State A is default.
    let mut state_a = State::new();
    // State B sets last node on the state.
    let mut state_b = State::new();
    state_b.set_last_node(true);
    // State C gets a "foobarbaz" prefix.
    let mut state_c = State::new();
    state_c.update(INPUT_PREFIX);
    // State D gets wacky parameters.
    let mut state_d = Params::new()
        .hash_length(18)
        .key(b"bar")
        .salt(b"bazbazbazbazbazb")
        .personal(b"bing bing bing b")
        .fanout(2)
        .max_depth(3)
        .max_leaf_length(0x04050607)
        .node_offset(0x08090a0b0c0d0e0f)
        .node_depth(16)
        .inner_hash_length(17)
        .last_node(true)
        .to_state();

    let mut input = [0; 35 * BLOCKBYTES];
    blake2bp::test::paint_input(&mut input);
    let input_e = &input[0_ * BLOCKBYTES..10 * BLOCKBYTES];
    let input_f = &input[10 * BLOCKBYTES..20 * BLOCKBYTES];
    let input_g = &input[20 * BLOCKBYTES..30 * BLOCKBYTES];
    // Input H is short.
    let input_h = &input[30 * BLOCKBYTES..35 * BLOCKBYTES];

    // Loop over four different permutations of the input.
    for (input0, input1, input2, input3) in &[
        (input_e, input_f, input_g, input_h),
        (input_f, input_g, input_h, input_e),
        (input_g, input_h, input_e, input_f),
        (input_h, input_e, input_f, input_g),
    ] {
        // For each input permutation, run four permutations of the states.
        test_run(
            &mut state_a,
            &mut state_b,
            &mut state_c,
            &mut state_d,
            input0,
            input1,
            input2,
            input3,
        );
        test_run(
            &mut state_b,
            &mut state_c,
            &mut state_d,
            &mut state_a,
            input0,
            input1,
            input2,
            input3,
        );
        test_run(
            &mut state_c,
            &mut state_d,
            &mut state_a,
            &mut state_b,
            input0,
            input1,
            input2,
            input3,
        );
        test_run(
            &mut state_d,
            &mut state_a,
            &mut state_b,
            &mut state_c,
            input0,
            input1,
            input2,
            input3,
        );
    }
}
