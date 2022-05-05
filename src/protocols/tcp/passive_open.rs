// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

use super::{constants::FALLBACK_MSS, established::ControlBlock, isn_generator::IsnGenerator};
use crate::{
    futures::FutureOperation,
    protocols::{
        arp::ArpPeer,
        ethernet2::{EtherType2, Ethernet2Header},
        ip::IpProtocol,
        ipv4::{Ipv4Endpoint, Ipv4Header},
        tcp::{
            established::cc::{self, CongestionControl},
            segment::{TcpHeader, TcpOptions2, TcpSegment},
            SeqNumber,
        },
    },
};
use ::scheduler::SchedulerHandle;
use ::futures::FutureExt;
use ::libc::{EBADMSG, ECONNREFUSED, ETIMEDOUT};
use ::runtime::{fail::Fail, memory::Buffer, Runtime};
use ::std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    convert::TryInto,
    future::Future,
    rc::Rc,
    task::{Context, Poll, Waker},
    time::Duration,
};

struct InflightAccept {
    local_isn: SeqNumber,
    remote_isn: SeqNumber,
    header_window_size: u16,
    remote_window_scale: Option<u8>,
    mss: usize,

    #[allow(unused)]
    handle: SchedulerHandle,
}

struct ReadySockets<RT: Runtime> {
    ready: VecDeque<Result<ControlBlock<RT>, Fail>>,
    endpoints: HashSet<Ipv4Endpoint>,
    waker: Option<Waker>,
}

impl<RT: Runtime> ReadySockets<RT> {
    fn push_ok(&mut self, cb: ControlBlock<RT>) {
        assert!(self.endpoints.insert(cb.get_remote()));
        self.ready.push_back(Ok(cb));
        if let Some(w) = self.waker.take() {
            w.wake()
        }
    }

    fn push_err(&mut self, err: Fail) {
        self.ready.push_back(Err(err));
        if let Some(w) = self.waker.take() {
            w.wake()
        }
    }

    fn poll(&mut self, ctx: &mut Context) -> Poll<Result<ControlBlock<RT>, Fail>> {
        let r = match self.ready.pop_front() {
            Some(r) => r,
            None => {
                self.waker.replace(ctx.waker().clone());
                return Poll::Pending;
            }
        };
        if let Ok(ref cb) = r {
            assert!(self.endpoints.remove(&cb.get_remote()));
        }
        Poll::Ready(r)
    }

    fn len(&self) -> usize {
        self.ready.len()
    }
}

pub struct PassiveSocket<RT: Runtime> {
    inflight: HashMap<Ipv4Endpoint, InflightAccept>,
    ready: Rc<RefCell<ReadySockets<RT>>>,

    max_backlog: usize,
    isn_generator: IsnGenerator,

    local: Ipv4Endpoint,
    rt: RT,
    arp: ArpPeer<RT>,
}

impl<RT: Runtime> PassiveSocket<RT> {
    pub fn new(local: Ipv4Endpoint, max_backlog: usize, rt: RT, arp: ArpPeer<RT>) -> Self {
        let ready = ReadySockets {
            ready: VecDeque::new(),
            endpoints: HashSet::new(),
            waker: None,
        };
        let ready = Rc::new(RefCell::new(ready));
        let nonce = rt.rng_gen();
        Self {
            inflight: HashMap::new(),
            ready,
            max_backlog,
            isn_generator: IsnGenerator::new(nonce),
            local,
            rt,
            arp,
        }
    }

    pub fn poll_accept(&mut self, ctx: &mut Context) -> Poll<Result<ControlBlock<RT>, Fail>> {
        self.ready.borrow_mut().poll(ctx)
    }

    pub fn receive(&mut self, ip_header: &Ipv4Header, header: &TcpHeader) -> Result<(), Fail> {
        let remote = Ipv4Endpoint::new(ip_header.get_src_addr(), header.src_port);
        if self.ready.borrow().endpoints.contains(&remote) {
            // TODO: What should we do if a packet shows up for a connection that hasn't been `accept`ed yet?
            return Ok(());
        }
        let inflight_len = self.inflight.len();

        // If the packet is for an inflight connection, route it there.
        if self.inflight.contains_key(&remote) {
            if !header.ack {
                return Err(Fail::new(EBADMSG, "expeting ACK"));
            }
            debug!("Received ACK: {:?}", header);
            let &InflightAccept {
                local_isn,
                remote_isn,
                header_window_size,
                remote_window_scale,
                mss,
                ..
            } = self.inflight.get(&remote).unwrap();
            if header.ack_num != local_isn + SeqNumber::from(1) {
                return Err(Fail::new(EBADMSG, "invalid SYN+ACK seq num"));
            }

            let tcp_options = self.rt.tcp_options();
            let (local_window_scale, remote_window_scale) = match remote_window_scale {
                Some(w) => (tcp_options.get_window_scale() as u32, w),
                None => (0, 0),
            };
            let remote_window_size = (header_window_size)
                .checked_shl(remote_window_scale as u32)
                .expect("TODO: Window size overflow")
                .try_into()
                .expect("TODO: Window size overflow");
            let local_window_size = (tcp_options.get_receive_window_size() as u32)
                .checked_shl(local_window_scale as u32)
                .expect("TODO: Window size overflow");
            info!(
                "Window sizes: local {}, remote {}",
                local_window_size, remote_window_size
            );
            info!(
                "Window scale: local {}, remote {}",
                local_window_scale, remote_window_scale
            );

            self.inflight.remove(&remote);
            let cb = ControlBlock::new(
                self.local,
                remote,
                self.rt.clone(),
                self.arp.clone(),
                remote_isn + SeqNumber::from(1),
                self.rt.tcp_options().get_ack_delay_timeout(),
                local_window_size,
                local_window_scale,
                local_isn + SeqNumber::from(1),
                remote_window_size,
                remote_window_scale,
                mss,
                cc::None::new,
                None,
            );
            self.ready.borrow_mut().push_ok(cb);
            return Ok(());
        }

        // Otherwise, start a new connection.
        if !header.syn || header.ack || header.rst {
            return Err(Fail::new(EBADMSG, "invalid flags"));
        }
        debug!("Received SYN: {:?}", header);
        if inflight_len + self.ready.borrow().len() >= self.max_backlog {
            // TODO: Should we send a RST here?
            return Err(Fail::new(ECONNREFUSED, "connection refused"));
        }
        let local_isn = self.isn_generator.generate(&self.local, &remote);
        let remote_isn = header.seq_num;
        let future = Self::background(
            local_isn,
            remote_isn,
            self.local,
            remote,
            self.rt.clone(),
            self.arp.clone(),
            self.ready.clone(),
        );
        let handle: SchedulerHandle = self
            .rt
            .spawn(FutureOperation::Background::<RT>(future.boxed_local()));

        let mut remote_window_scale = None;
        let mut mss = FALLBACK_MSS;
        for option in header.iter_options() {
            match option {
                TcpOptions2::WindowScale(w) => {
                    info!("Received window scale: {:?}", w);
                    remote_window_scale = Some(*w);
                }
                TcpOptions2::MaximumSegmentSize(m) => {
                    info!("Received advertised MSS: {}", m);
                    mss = *m as usize;
                }
                _ => continue,
            }
        }
        let accept = InflightAccept {
            local_isn,
            remote_isn,
            header_window_size: header.window_size,
            remote_window_scale,
            mss,
            handle,
        };
        self.inflight.insert(remote, accept);
        Ok(())
    }

    fn background(
        local_isn: SeqNumber,
        remote_isn: SeqNumber,
        local: Ipv4Endpoint,
        remote: Ipv4Endpoint,
        rt: RT,
        arp: ArpPeer<RT>,
        ready: Rc<RefCell<ReadySockets<RT>>>,
    ) -> impl Future<Output = ()> {
        let tcp_options = rt.tcp_options();
        let handshake_retries: usize = tcp_options.get_handshake_retries();
        let handshake_timeout: Duration = tcp_options.get_handshake_timeout();

        async move {
            for _ in 0..handshake_retries {
                let remote_link_addr = match arp.query(remote.get_address()).await {
                    Ok(r) => r,
                    Err(e) => {
                        warn!("ARP query failed: {:?}", e);
                        continue;
                    }
                };
                let mut tcp_hdr = TcpHeader::new(local.get_port(), remote.get_port());
                tcp_hdr.syn = true;
                tcp_hdr.seq_num = local_isn;
                tcp_hdr.ack = true;
                tcp_hdr.ack_num = remote_isn + SeqNumber::from(1);
                tcp_hdr.window_size = tcp_options.get_receive_window_size();

                let mss = tcp_options.get_advertised_mss() as u16;
                tcp_hdr.push_option(TcpOptions2::MaximumSegmentSize(mss));
                info!("Advertising MSS: {}", mss);

                tcp_hdr.push_option(TcpOptions2::WindowScale(tcp_options.get_window_scale()));
                info!(
                    "Advertising window scale: {}",
                    tcp_options.get_window_scale()
                );

                debug!("Sending SYN+ACK: {:?}", tcp_hdr);
                let segment = TcpSegment {
                    ethernet2_hdr: Ethernet2Header::new(
                        remote_link_addr,
                        rt.local_link_addr(),
                        EtherType2::Ipv4,
                    ),
                    ipv4_hdr: Ipv4Header::new(
                        local.get_address(),
                        remote.get_address(),
                        IpProtocol::TCP,
                    ),
                    tcp_hdr,
                    data: RT::Buf::empty(),
                    tx_checksum_offload: tcp_options.get_rx_checksum_offload(),
                };
                rt.transmit(segment);
                rt.wait(handshake_timeout).await;
            }
            ready
                .borrow_mut()
                .push_err(Fail::new(ETIMEDOUT, "handshake timeout"));
        }
    }
}
