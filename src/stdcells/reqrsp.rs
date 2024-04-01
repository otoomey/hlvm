use std::marker::PhantomData;

use super::Port;

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ReqPort<T> {
    pub valid: bool,
    pub data: T,
    pub port: Port
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RspPort<T> {
    pub ready: bool,
    pub port: Port,
    pub phantom: PhantomData<T>
}

impl<T> ReqPort<T> {
    pub fn new(data: T) -> Self {
        ReqPort {
            valid: false,
            data,
            port: Port::Z
        }
    }
}

impl<T> RspPort<T> {
    pub fn new() -> Self {
        RspPort {
            ready: false,
            port: Port::Z,
            phantom: PhantomData
        }
    }
}