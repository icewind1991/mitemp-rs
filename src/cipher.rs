use std::ops::Rem;

pub fn mix_a(mac: [u8; 6], product_id: u16) -> [u8; 8] {
    [
        mac[0],
        mac[2],
        mac[5],
        (product_id & 0xFF) as u8,
        (product_id & 0xFF) as u8,
        mac[4],
        mac[5],
        mac[1],
    ]
}

pub fn mix_b(mac: [u8; 6], product_id: u16) -> [u8; 8] {
    [
        mac[0],
        mac[2],
        mac[5],
        (product_id >> 8 & 255) as u8,
        mac[4],
        mac[0],
        mac[5],
        (product_id & 255) as u8,
    ]
}

const KEY_SIZE: usize = 256;

fn cipher_init(key: &[u8]) -> [u8; KEY_SIZE] {
    let mut perm = [0u8; KEY_SIZE];
    for i in 0..KEY_SIZE {
        perm[i] = i as u8;
    }

    let mut j: u8 = 0;
    for ia in 0..KEY_SIZE {
        j = j.wrapping_add(perm[ia]).wrapping_add(key[ia % key.len()]);
        perm.swap(ia, j as usize);
    }
    perm
}

fn cipher_crypt(input: &[u8], mut perm: [u8; KEY_SIZE]) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len());
    let mut index1: u8 = 0;
    let mut index2: u8 = 0;

    for i in 0..input.len() {
        index1 = index1.wrapping_add(1);
        index2 = index2.wrapping_add(perm[index1 as usize]);
        perm.swap(index1 as usize, index2 as usize);
        let index = perm[index1 as usize].wrapping_add(perm[index2 as usize]);
        output.push(input[i] ^ perm[index as usize]);
    }
    output
}

pub fn cipher(key: &[u8], input: &[u8]) -> Vec<u8> {
    cipher_crypt(input, cipher_init(key))
}
