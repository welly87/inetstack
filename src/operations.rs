// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

//==============================================================================
// Imports
//==============================================================================

use crate::protocols::ipv4::Ipv4Endpoint;
use ::runtime::{
    fail::Fail,
    memory::Buffer,
    QDesc,
};
use ::std::fmt;

//==============================================================================
// Structures
//==============================================================================

pub enum OperationResult<T: Buffer> {
    Connect,
    Accept(QDesc),
    Push,
    Pop(Option<Ipv4Endpoint>, T),
    Failed(Fail),
}

//==============================================================================
// Trait Implementations
//==============================================================================

impl<T: Buffer> fmt::Debug for OperationResult<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OperationResult::Connect => write!(f, "Connect"),
            OperationResult::Accept(..) => write!(f, "Accept"),
            OperationResult::Push => write!(f, "Push"),
            OperationResult::Pop(..) => write!(f, "Pop"),
            OperationResult::Failed(ref e) => write!(f, "Failed({:?})", e),
        }
    }
}
