mod selftest;

use hex_literal::hex;
use selftest::Tester;
use sha2::{Digest, Sha256};

const INPUT_DATA: &str = include_str!("boards.fen");
const OUTPUT_HASH: [u8; 32] =
    hex!("1ac232af9c1ede66b0cf423c87838324b09d178a5721b2c4ded7d87540a96318");

#[ignore]
#[test]
fn test_selftest() {
    let mut hasher = Sha256::default();
    let mut tester = Tester::new(Default::default(), &mut hasher);
    tester.run_many(&mut INPUT_DATA.as_bytes());
    assert_eq!(&hasher.finalize()[..], &OUTPUT_HASH[..]);
}
