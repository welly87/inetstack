#![feature(new_uninit)]
#![feature(const_panic, const_alloc_layout)]
#![feature(const_mut_refs, const_type_name)]
#![feature(maybe_uninit_uninit_array, maybe_uninit_extra, maybe_uninit_ref)]

mod common;

mod tcp_migration {
    use crate::common::{
        arp, libos::*, runtime::DummyRuntime, ALICE_IPV4, ALICE_MAC, BOB_IPV4, BOB_MAC, PORT_BASE,
    };
    use ::catnip::{
        libos::LibOS,
        protocols::{ip, ipv4::Ipv4Endpoint},
    };
    use ::crossbeam_channel::{self};
    use ::runtime::fail::Fail;
    use ::runtime::memory::MemoryRuntime;
    use ::runtime::queue::IoQueueDescriptor;
    use ::runtime::types::dmtr_opcode_t;
    use ::std::{convert::TryFrom, net::Ipv4Addr, thread};
    use libc;

    fn do_tcp_push_remote(port: u16) {
        let (alice_tx, alice_rx) = crossbeam_channel::unbounded();
        let (bob_tx, bob_rx) = crossbeam_channel::unbounded();

        let alice = thread::spawn(move || {
            let mut libos = DummyLibOS::new(ALICE_MAC, ALICE_IPV4, alice_tx, bob_rx, arp());

            let port = ip::Port::try_from(port).unwrap();
            let local = Ipv4Endpoint::new(ALICE_IPV4, port);

            // Open connection.
            let sockfd = libos.socket(libc::AF_INET, libc::SOCK_STREAM, 0).unwrap();
            libos.bind(sockfd, local).unwrap();
            libos.listen(sockfd, 8).unwrap();
            let qt = libos.accept(sockfd).unwrap();
            let r = libos.wait(qt);
            assert_eq!(r.qr_opcode, dmtr_opcode_t::DMTR_OPC_ACCEPT);

            // Pop data.
            let qd = IoQueueDescriptor::from(unsafe { r.qr_value.ares.qd } as usize);
            let qt = libos.pop(qd).unwrap();
            let qr = libos.wait(qt);
            assert_eq!(qr.qr_opcode, dmtr_opcode_t::DMTR_OPC_POP);

            // Sanity check data.
            let sga = unsafe { qr.qr_value.sga };
            DummyLibOS::check_data(sga);
            libos.rt().free_sgarray(sga);

            libos.get_tcp_state(qd).expect("Unable to get TCP state");

            // Close connection.
            libos.close(qd).unwrap();
            libos.close(sockfd).unwrap_err();
        });

        let bob = thread::spawn(move || {
            let mut libos = DummyLibOS::new(BOB_MAC, BOB_IPV4, bob_tx, alice_rx, arp());

            let port = ip::Port::try_from(port).unwrap();
            let remote = Ipv4Endpoint::new(ALICE_IPV4, port);

            // Open connection.
            let sockfd = libos.socket(libc::AF_INET, libc::SOCK_STREAM, 0).unwrap();
            let qt = libos.connect(sockfd, remote).unwrap();
            assert_eq!(libos.wait(qt).qr_opcode, dmtr_opcode_t::DMTR_OPC_CONNECT);

            // Cook some data.
            let body_sga = DummyLibOS::cook_data(&mut libos, 32);

            // Push data.
            let qt = libos.push(sockfd, &body_sga).unwrap();
            assert_eq!(libos.wait(qt).qr_opcode, dmtr_opcode_t::DMTR_OPC_PUSH);
            libos.rt().free_sgarray(body_sga);

            // Close connection.
            libos.close(sockfd).unwrap();
        });

        alice.join().unwrap();
        bob.join().unwrap();
    }

    #[test]
    fn catnip_tcp_push_remote() {

        do_tcp_push_remote(PORT_BASE + 100)
    }


}