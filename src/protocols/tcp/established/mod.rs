// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

mod background;
pub mod congestion_control;
mod ctrlblk;
mod rto;
mod sender;

pub use self::ctrlblk::{
    ControlBlock,
    State,
};

use self::background::background;
use crate::{
    futures::FutureOperation,
    protocols::{
        ipv4::Ipv4Endpoint,
        tcp::segment::TcpHeader,
    },
};
use ::futures::{
    channel::mpsc,
    FutureExt,
};
use ::runtime::{
    fail::Fail,
    QDesc,
    Runtime,
};
use ::scheduler::SchedulerHandle;
use ::std::{
    rc::Rc,
    task::{
        Context,
        Poll,
    },
    time::Duration,
};

pub struct EstablishedSocket<RT: Runtime> {
    pub cb: Rc<ControlBlock<RT>>,
    #[allow(unused)]
    background_work: SchedulerHandle,
}

impl<RT: Runtime> EstablishedSocket<RT> {
    pub fn new(cb: ControlBlock<RT>, fd: QDesc, dead_socket_tx: mpsc::UnboundedSender<QDesc>) -> Self {
        let cb = Rc::new(cb);
        let future = background(cb.clone(), fd, dead_socket_tx);
        let handle: SchedulerHandle = cb.rt().spawn(FutureOperation::Background::<RT>(future.boxed_local()));
        Self {
            cb: cb.clone(),
            background_work: handle,
        }
    }

    pub fn receive(&self, header: &mut TcpHeader, data: RT::Buf) {
        self.cb.receive(header, data)
    }

    pub fn send(&self, buf: RT::Buf) -> Result<(), Fail> {
        self.cb.send(buf)
    }

    pub fn poll_recv(&self, ctx: &mut Context) -> Poll<Result<RT::Buf, Fail>> {
        self.cb.poll_recv(ctx)
    }

    pub fn close(&self) -> Result<(), Fail> {
        self.cb.close()
    }

    pub fn remote_mss(&self) -> usize {
        self.cb.remote_mss()
    }

    pub fn current_rto(&self) -> Duration {
        self.cb.rto_estimate()
    }

    pub fn endpoints(&self) -> (Ipv4Endpoint, Ipv4Endpoint) {
        (self.cb.get_local(), self.cb.get_remote())
    }
}
