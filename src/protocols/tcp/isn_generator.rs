// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

use crate::protocols::{ipv4::Ipv4Endpoint, tcp::SeqNumber};
#[allow(unused_imports)]
use crc::{crc32, Hasher32};
#[allow(unused_imports)]
use std::{hash::Hasher, num::Wrapping};

#[allow(dead_code)]
pub struct IsnGenerator {
    nonce: u32,
    counter: Wrapping<u16>,
}

impl IsnGenerator {
    pub fn new(nonce: u32) -> Self {
        Self {
            nonce,
            counter: Wrapping(0),
        }
    }

    #[cfg(test)]
    pub fn generate(&mut self, _local: &Ipv4Endpoint, _remote: &Ipv4Endpoint) -> SeqNumber {
        SeqNumber::from(0)
    }

    #[cfg(not(test))]
    pub fn generate(&mut self, local: &Ipv4Endpoint, remote: &Ipv4Endpoint) -> SeqNumber {
        let mut hash = crc32::Digest::new(crc32::IEEE);
        hash.write_u32(remote.get_address().into());
        hash.write_u16(remote.get_port().into());
        hash.write_u32(local.get_address().into());
        hash.write_u16(local.get_port().into());
        hash.write_u32(self.nonce);
        let hash = hash.sum32();
        let isn = SeqNumber::from(hash + self.counter.0 as u32);
        self.counter += Wrapping(1);
        isn
    }
}
