// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

pub mod engine;
pub mod runtime;

pub use self::runtime::TestRuntime;
pub use engine::Engine;

use ::runtime::network::{
    config::{ArpConfig, TcpConfig, UdpConfig},
    types::MacAddress,
};
use ::std::{
    collections::HashMap,
    net::Ipv4Addr,
    time::{Duration, Instant},
};

//==============================================================================
// Constants
//==============================================================================

pub const RECEIVE_WINDOW_SIZE: usize = 1024;
pub const ALICE_MAC: MacAddress = MacAddress::new([0x12, 0x23, 0x45, 0x67, 0x89, 0xab]);
pub const ALICE_IPV4: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 1);
pub const BOB_MAC: MacAddress = MacAddress::new([0xab, 0x89, 0x67, 0x45, 0x23, 0x12]);
pub const BOB_IPV4: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 2);
pub const CARRIE_MAC: MacAddress = MacAddress::new([0xef, 0xcd, 0xab, 0x89, 0x67, 0x45]);
pub const CARRIE_IPV4: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 3);
pub const JUAN_MAC: MacAddress = MacAddress::new([0x18, 0x32, 0xef, 0xde, 0xad, 0xff]);
pub const JUAN_IPV4: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 69);


//==============================================================================
// Types
//==============================================================================

pub type TestEngine = Engine<TestRuntime>;

//==============================================================================
// Standalone Functions
//==============================================================================

pub fn new_alice(now: Instant) -> Engine<TestRuntime> {
    let arp_options = ArpConfig::new(
        Some(Duration::from_secs(600)),
        Some(Duration::from_secs(1)),
        Some(2),
        Some(HashMap::new()),
        Some(false),
    );
    let udp_options = UdpConfig::default();
    let tcp_options = TcpConfig::default();
    let rt = TestRuntime::new(
        "alice",
        now,
        arp_options,
        udp_options,
        tcp_options,
        ALICE_MAC,
        ALICE_IPV4,
    );
    Engine::new(rt).unwrap()
}

pub fn new_bob(now: Instant) -> Engine<TestRuntime> {
    let arp_options = ArpConfig::new(
        Some(Duration::from_secs(600)),
        Some(Duration::from_secs(1)),
        Some(2),
        Some(HashMap::new()),
        Some(false),
    );
    let udp_options = UdpConfig::default();
    let tcp_options = TcpConfig::default();
    let rt = TestRuntime::new(
        "bob",
        now,
        arp_options,
        udp_options,
        tcp_options,
        BOB_MAC,
        BOB_IPV4,
    );
    Engine::new(rt).unwrap()
}

pub fn new_alice2(now: Instant) -> Engine<TestRuntime> {
    let mut arp: HashMap<Ipv4Addr, MacAddress> = HashMap::<Ipv4Addr, MacAddress>::new();
    arp.insert(ALICE_IPV4, ALICE_MAC);
    arp.insert(BOB_IPV4, BOB_MAC);
    let arp_options = ArpConfig::new(
        Some(Duration::from_secs(600)),
        Some(Duration::from_secs(1)),
        Some(2),
        Some(arp),
        Some(false),
    );
    let udp_options = UdpConfig::default();
    let tcp_options = TcpConfig::default();
    let rt = TestRuntime::new(
        "alice",
        now,
        arp_options,
        udp_options,
        tcp_options,
        ALICE_MAC,
        ALICE_IPV4,
    );
    Engine::new(rt).unwrap()
}

pub fn new_bob2(now: Instant) -> Engine<TestRuntime> {
    let mut arp: HashMap<Ipv4Addr, MacAddress> = HashMap::<Ipv4Addr, MacAddress>::new();
    arp.insert(BOB_IPV4, BOB_MAC);
    arp.insert(ALICE_IPV4, ALICE_MAC);
    let arp_options = ArpConfig::new(
        Some(Duration::from_secs(600)),
        Some(Duration::from_secs(1)),
        Some(2),
        Some(arp),
        Some(false),
    );
    let udp_options = UdpConfig::default();
    let tcp_options = TcpConfig::default();
    let rt = TestRuntime::new(
        "bob",
        now,
        arp_options,
        udp_options,
        tcp_options,
        BOB_MAC,
        BOB_IPV4,
    );
    Engine::new(rt).unwrap()
}

pub fn new_carrie(now: Instant) -> Engine<TestRuntime> {
    let arp_options = ArpConfig::new(
        Some(Duration::from_secs(600)),
        Some(Duration::from_secs(1)),
        Some(2),
        Some(HashMap::new()),
        Some(false),
    );
    let udp_options = UdpConfig::default();
    let tcp_options = TcpConfig::default();

    let rt = TestRuntime::new(
        "carrie",
        now,
        arp_options,
        udp_options,
        tcp_options,
        CARRIE_MAC,
        CARRIE_IPV4,
    );
    Engine::new(rt).unwrap()
}

pub fn new_juan(now: Instant) -> Engine<TestRuntime> {
    let mut arp: HashMap<Ipv4Addr, MacAddress> = HashMap::<Ipv4Addr, MacAddress>::new();
    // arp.insert(J, BOB_MAC);

    arp.insert(ALICE_IPV4, ALICE_MAC);
    let arp_options = ArpConfig::new(
        Some(Duration::from_secs(600)),
        Some(Duration::from_secs(1)),
        Some(2),
        Some(arp),
        Some(false),
    );
    let udp_options = UdpConfig::default();
    let tcp_options = TcpConfig::default();
    let rt = TestRuntime::new(
        "juan",
        now,
        arp_options,
        udp_options,
        tcp_options,
        JUAN_MAC,
        JUAN_IPV4,
    );
    Engine::new(rt).unwrap()
}
