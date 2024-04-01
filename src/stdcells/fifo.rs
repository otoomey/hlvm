use std::{borrow::Cow, collections::VecDeque};

use super::{reqrsp::{ReqPort, RspPort}, Node};

#[derive(Clone)]
pub struct Fifo<T: Clone + 'static + Eq + PartialEq + Default> {
    buffer: VecDeque<T>,
    zero_cycle: bool,
    pub req: ReqPort<T>,
    pub rsp: RspPort<T>,
}

impl<T: Clone + 'static + Eq + PartialEq + Default> Fifo<T> {
    pub fn new(capacity: usize, zero_cycle: bool) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            zero_cycle,
            req: ReqPort::new(Default::default()),
            rsp: RspPort::new()
        }
    }
}

impl<T: Clone + 'static + Eq + PartialEq + Default> Node for Fifo<T> {
    fn get_port_ids(&self) -> std::borrow::Cow<[super::Port]> {
        Cow::from(vec![self.rsp.port, self.req.port])
    }

    fn get_port(&self, port: usize) -> &dyn std::any::Any {
        match port {
            0 => &self.rsp,
            1 => &self.req,
            _ => { panic!("Port {} is out of range for fifo; expected 0 or 1.", port) }
        }
    }

    fn csim<'a>(&mut self, ctx: &super::Ctx<'a, Self>) -> bool {
        self.rsp.ready = self.buffer.len() < self.buffer.capacity();
        
        let ivalid = ctx.port::<ReqPort<Option<T>>>(self.rsp.port)
            .map(|req| req.valid)
            .unwrap_or(false);

        let idata = ctx.port::<ReqPort<Option<T>>>(self.rsp.port)
            .map(|req| req.data.as_ref().map(|x| x.clone()))
            .flatten()
            .unwrap_or_default();

        self.req.valid = self.buffer.len() > 0 || (ivalid && self.zero_cycle);

        self.req.data = match self.zero_cycle && self.buffer.is_empty() {
            true => idata,
            false => self.buffer.front()
                .map(|x| x.clone())
                .unwrap_or_default()
        };

        (ctx.state().rsp != self.rsp) || (ctx.state().req != self.req)
    }

    fn edge<'a>(&mut self, ctx: &super::Ctx<'a, Self>) {
        
        let pop = if let Some(port) = ctx.port::<RspPort<T>>(self.req.port) {
            if self.req.valid && port.ready {
                self.buffer.pop_front();
                true
            } else {
                false
            }
        } else { false };

        if let Some(port) = ctx.port::<ReqPort<T>>(self.rsp.port) {
            if port.valid && self.rsp.ready {
                self.buffer.push_back(port.data.clone());
            }
        }
    }
}
