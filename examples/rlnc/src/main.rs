use rand::RngExt;

struct EncodedPacket {
    coeffs: Vec<u8>,
    payload: Vec<u8>,
}

fn split(data: Vec<u8>, chunk_size: usize) -> Vec<Vec<u8>> {
    let mut packets = Vec::new();

    for chunk in data.chunks(chunk_size) {
        let mut p = chunk.to_vec();

        while p.len() < chunk_size {
            p.push(0); // padding
        }

        packets.push(p);
    }

    packets
}
fn gf_mul(mut a: u8, mut b: u8) -> u8 {
    let mut p = 0;

    for _ in 0..8 {
        if b & 1 != 0 {
            p ^= a;
        }

        let carry = a & 0x80;
        a <<= 1;

        if carry != 0 {
            a ^= 0x1b;
        }

        b >>= 1;
    }

    p
}

fn encode(packets: &[Vec<u8>]) -> EncodedPacket {
    fn gf_add(a: u8, b: u8) -> u8 {
        a ^ b
    }

    let mut rng = rand::rng();

    let n = packets.len();
    let size = packets[0].len();

    let mut coeffs = vec![0u8; n];
    let mut payload = vec![0u8; size];

    for i in 0..n {
        let coeff = rng.random::<u8>();
        coeffs[i] = coeff;

        for j in 0..size {
            let val = gf_mul(coeff, packets[i][j]);
            payload[j] = gf_add(payload[j], val);
        }
    }

    EncodedPacket { coeffs, payload }
}

fn generate(packets: &[Vec<u8>], count: usize) -> Vec<EncodedPacket> {
    (0..count).map(|_| encode(packets)).collect()
}

fn build_system(packets: &[EncodedPacket]) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
    let mut matrix = Vec::new();
    let mut data = Vec::new();

    for pkt in packets {
        matrix.push(pkt.coeffs.clone());
        data.push(pkt.payload.clone());
    }

    (matrix, data)
}

fn gf_inv(x: u8) -> u8 {
    for i in 1..=255 {
        if gf_mul(x, i) == 1 {
            return i;
        }
    }
    panic!("no inverse");
}

fn gaussian_elimination(matrix: &mut Vec<Vec<u8>>, data: &mut Vec<Vec<u8>>) {
    let n = matrix.len();

    for i in 0..n {
        // find pivot
        let mut pivot = i;
        while pivot < n && matrix[pivot][i] == 0 {
            pivot += 1;
        }

        if pivot == n {
            continue; // singular
        }

        matrix.swap(i, pivot);
        data.swap(i, pivot);

        // normalize pivot row
        let inv = gf_inv(matrix[i][i]); // we’ll define this next

        for j in 0..n {
            matrix[i][j] = gf_mul(matrix[i][j], inv);
        }

        for j in 0..data[i].len() {
            data[i][j] = gf_mul(data[i][j], inv);
        }

        // eliminate below
        for k in 0..n {
            if k != i {
                let factor = matrix[k][i];

                for j in 0..n {
                    let val = gf_mul(factor, matrix[i][j]);
                    matrix[k][j] ^= val;
                }

                for j in 0..data[k].len() {
                    let val = gf_mul(factor, data[i][j]);
                    data[k][j] ^= val;
                }
            }
        }
    }
}

fn reconstruct(data: Vec<Vec<u8>>, original_len: usize) -> Vec<u8> {
    let mut result = Vec::new();

    for packet in data {
        result.extend(packet);
    }

    result.truncate(original_len); // remove padding
    result
}

/// original-data
/// split
/// encode (generate N packets)
/// pick N independent packets
/// decode (gaussian elimination)
/// reconstruct
/// == original ?
pub fn rlnc() {
    let input = b"1234hello_world__is_rlnc".to_vec();
    let original_len = input.len();

    println!(
        "Original: {:?},\nBytes: {:?}",
        String::from_utf8_lossy(&input),
        input
    );

    // split
    let packets = split(input.clone(), 5);
    let generation_size = packets.len();

    println!("Packets:");
    for p in &packets {
        println!("{:?}", p);
    }

    // encode (generate more than needed)
    let encoded = generate(&packets, generation_size + 2);

    println!("\nEncoded packets:");
    for e in &encoded {
        println!("coeffs: {:?}, payload: {:?}", e.coeffs, e.payload);
    }

    // Take first N packets (this should be randomized later)
    let selected: Vec<_> = encoded.into_iter().take(generation_size).collect();

    // build system
    let (mut matrix, mut data) = build_system(&selected);

    println!("\nMatrix before elimination:");
    println!("{:?}", matrix);

    // decode
    gaussian_elimination(&mut matrix, &mut data);

    println!("\nMatrix after elimination:");
    println!("{:?}", matrix);

    println!("\nDecoded packets:");
    for d in &data {
        println!("{:?}", d);
    }

    // reconstruct
    let output = reconstruct(data, original_len);

    println!("\nReconstructed: {:?}", String::from_utf8_lossy(&output));

    // verify
    assert_eq!(input, output);
    println!("\n✅ SUCCESS: Data reconstructed correctly");
}

fn main() {
    rlnc();
}
