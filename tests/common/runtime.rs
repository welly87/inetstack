// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

//==============================================================================
// Imports
//==============================================================================

use ::arrayvec::ArrayVec;
use ::crossbeam_channel;
use ::runtime::{
    memory::{
        Buffer,
        DataBuffer,
    },
    network::{
        config::{
            ArpConfig,
            TcpConfig,
            UdpConfig,
        },
        consts::RECEIVE_BATCH_SIZE,
        types::{
            Ipv4Addr,
            MacAddress,
        },
        NetworkRuntime,
        PacketBuf,
    },
    scheduler::Scheduler,
    timer::{
        Timer,
        TimerRc,
    },
};
use ::std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    time::{
        Duration,
        Instant,
    },
};

//==============================================================================
// Structures
//==============================================================================

/// Shared Dummy Runtime
struct SharedDummyRuntime {
    /// Random Number Generator
    /// Incoming Queue of Packets
    incoming: crossbeam_channel::Receiver<DataBuffer>,
    /// Outgoing Queue of Packets
    outgoing: crossbeam_channel::Sender<DataBuffer>,
}

/// Dummy Runtime
#[derive(Clone)]
pub struct DummyRuntime {
    /// Shared Member Fields
    inner: Rc<RefCell<SharedDummyRuntime>>,
    pub scheduler: Scheduler,
    pub clock: TimerRc,
    link_addr: MacAddress,
    ipv4_addr: Ipv4Addr,
    tcp_options: TcpConfig,
    arp_options: ArpConfig,
}

//==============================================================================
// Associate Functions
//==============================================================================

/// Associate Functions for Dummy Runtime
impl DummyRuntime {
    /// Creates a Dummy Runtime.
    pub fn new(
        now: Instant,
        link_addr: MacAddress,
        ipv4_addr: Ipv4Addr,
        incoming: crossbeam_channel::Receiver<DataBuffer>,
        outgoing: crossbeam_channel::Sender<DataBuffer>,
        arp: HashMap<Ipv4Addr, MacAddress>,
    ) -> Self {
        let arp_options: ArpConfig = ArpConfig::new(
            Some(Duration::from_secs(600)),
            Some(Duration::from_secs(1)),
            Some(2),
            Some(arp),
            Some(false),
        );

        let inner = SharedDummyRuntime { incoming, outgoing };
        Self {
            inner: Rc::new(RefCell::new(inner)),
            scheduler: Scheduler::default(),
            clock: TimerRc(Rc::new(Timer::new(now))),
            link_addr,
            ipv4_addr,
            tcp_options: TcpConfig::default(),
            arp_options,
        }
    }
}

//==============================================================================
// Trait Implementations
//==============================================================================

/// Network Runtime Trait Implementation for Dummy Runtime
impl NetworkRuntime for DummyRuntime {
    fn transmit(&self, pkt: impl PacketBuf) {
        let header_size = pkt.header_size();
        let body_size = pkt.body_size();

        let mut buf: DataBuffer = DataBuffer::new(header_size + body_size).unwrap();
        pkt.write_header(&mut buf[..header_size]);
        if let Some(body) = pkt.take_body() {
            buf[header_size..].copy_from_slice(&body[..]);
        }
        self.inner.borrow_mut().outgoing.try_send(buf).unwrap();
    }

    fn receive(&self) -> ArrayVec<Buffer, RECEIVE_BATCH_SIZE> {
        let mut out = ArrayVec::new();
        if let Some(buf) = self.inner.borrow_mut().incoming.try_recv().ok() {
            let dbuf: Buffer = Buffer::Heap(buf);
            out.push(dbuf);
        }
        out
    }

    fn local_link_addr(&self) -> MacAddress {
        self.link_addr.clone()
    }

    fn local_ipv4_addr(&self) -> Ipv4Addr {
        self.ipv4_addr.clone()
    }

    fn tcp_options(&self) -> TcpConfig {
        self.tcp_options.clone()
    }

    fn udp_options(&self) -> UdpConfig {
        UdpConfig::default()
    }

    fn arp_options(&self) -> ArpConfig {
        self.arp_options.clone()
    }
}
