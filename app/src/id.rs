use rand::Rng;
use rand::{rngs::StdRng, SeedableRng};

const ID_LENGTH: usize = 21;
const ALPHABET: [char; 64] = [
    '_', '-', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g',
    'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

pub fn generate_id() -> String {
    assert!(ALPHABET.len() <= u8::max_value() as usize);
    let mask = ALPHABET.len().next_power_of_two() - 1;
    assert!(ALPHABET.len() <= mask + 1);
    // Don't know what these magic numbers do
    let step = 8 * ID_LENGTH / 5;
    let mut id = String::with_capacity(ID_LENGTH);

    loop {
        let mut rng = StdRng::from_entropy();
        let mut bytes = vec![0; step];
        rng.fill(&mut bytes[..]);

        for &byte in &bytes {
            let byte = byte as usize & mask;

            if byte < ALPHABET.len() {
                id.push(ALPHABET[byte]);
            }

            if id.len() == ID_LENGTH {
                return id;
            }
        }
    }
}
